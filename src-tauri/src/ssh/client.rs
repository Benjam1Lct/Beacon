use std::sync::{Arc, Mutex};
use std::time::Duration;

use russh::client::{self, Handler};
use russh::ChannelMsg;
use serde::{Deserialize, Serialize};

/// Profil de connexion résolu (secrets déjà chargés depuis le keyring / le fichier).
#[derive(Debug, Clone, Deserialize)]
pub struct SshProfile {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub auth: AuthInput,
}

fn default_port() -> u16 {
    22
}

/// Méthode d'authentification.
///
/// - `Key { path }` : charge la clé depuis un fichier (flux d'import).
/// - `KeyContent { pem }` : clé déjà en mémoire (chargée depuis le keyring).
/// - `Password` : bootstrap uniquement, jamais persisté.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AuthInput {
    Password {
        password: String,
    },
    Key {
        path: String,
        passphrase: Option<String>,
    },
    KeyContent {
        pem: String,
        passphrase: Option<String>,
    },
}

/// Résultat d'une commande distante.
#[derive(Debug, Serialize)]
pub struct SshResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u32,
}

/// Résultat d'une connexion : sortie + empreinte de la clé d'hôte observée.
#[derive(Debug, Serialize)]
pub struct ExecOutcome {
    pub result: SshResult,
    pub host_key_fp: String,
}

/// Erreurs de connexion, sérialisées en message clair pour le frontend.
#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("Connexion impossible à {0} : {1}")]
    Connect(String, String),
    #[error("Authentification refusée (vérifie l'utilisateur et la clé/mot de passe)")]
    Auth,
    #[error("Clé SSH illisible : {0}")]
    Key(String),
    #[error("Délai de connexion dépassé")]
    Timeout,
    #[error(
        "La clé d'hôte du serveur a changé ! Attendu {expected}, reçu {got}. \
         Possible attaque de l'intermédiaire (MITM) — connexion bloquée."
    )]
    HostKeyChanged { expected: String, got: String },
    #[error("Erreur SSH : {0}")]
    Protocol(String),
}

impl Serialize for SshError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

#[derive(Default)]
struct HostKeyState {
    seen_fp: Option<String>,
    expected: Option<String>,
    mismatch: bool,
}

/// Handler qui capture l'empreinte de la clé d'hôte et applique le pinning TOFU.
struct ClientHandler {
    state: Arc<Mutex<HostKeyState>>,
}

#[async_trait::async_trait]
impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let fp = server_public_key.fingerprint();
        let mut st = self.state.lock().unwrap();
        st.seen_fp = Some(fp.clone());
        if let Some(expected) = &st.expected {
            if expected != &fp {
                st.mismatch = true;
                return Ok(false); // rejette la connexion : clé d'hôte différente
            }
        }
        Ok(true)
    }
}

/// Se connecte, applique le pinning TOFU (`expected_fp`), exécute `command`, renvoie la sortie
/// et l'empreinte de la clé d'hôte observée.
pub async fn exec(
    profile: &SshProfile,
    command: &str,
    expected_fp: Option<&str>,
) -> Result<ExecOutcome, SshError> {
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        ..Default::default()
    });

    let state = Arc::new(Mutex::new(HostKeyState {
        expected: expected_fp.map(|s| s.to_string()),
        ..Default::default()
    }));
    let handler = ClientHandler {
        state: state.clone(),
    };

    let addr = (profile.host.as_str(), profile.port);
    let connect = client::connect(config, addr, handler);
    let mut session = match tokio::time::timeout(Duration::from_secs(15), connect).await {
        Err(_) => return Err(SshError::Timeout),
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            // Distingue le changement de clé d'hôte des autres erreurs de connexion.
            let st = state.lock().unwrap();
            if st.mismatch {
                return Err(SshError::HostKeyChanged {
                    expected: st.expected.clone().unwrap_or_default(),
                    got: st.seen_fp.clone().unwrap_or_default(),
                });
            }
            return Err(SshError::Connect(profile.host.clone(), e.to_string()));
        }
    };

    let authenticated = match &profile.auth {
        AuthInput::Password { password } => session
            .authenticate_password(&profile.username, password)
            .await
            .map_err(|e| SshError::Protocol(e.to_string()))?,
        AuthInput::Key { path, passphrase } => {
            let key = russh_keys::load_secret_key(path, passphrase.as_deref())
                .map_err(|e| SshError::Key(e.to_string()))?;
            session
                .authenticate_publickey(&profile.username, Arc::new(key))
                .await
                .map_err(|e| SshError::Protocol(e.to_string()))?
        }
        AuthInput::KeyContent { pem, passphrase } => {
            let key = russh_keys::decode_secret_key(pem, passphrase.as_deref())
                .map_err(|e| SshError::Key(e.to_string()))?;
            session
                .authenticate_publickey(&profile.username, Arc::new(key))
                .await
                .map_err(|e| SshError::Protocol(e.to_string()))?
        }
    };

    if !authenticated {
        return Err(SshError::Auth);
    }

    let mut channel = session
        .channel_open_session()
        .await
        .map_err(|e| SshError::Protocol(e.to_string()))?;
    channel
        .exec(true, command)
        .await
        .map_err(|e| SshError::Protocol(e.to_string()))?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_code: u32 = 0;

    while let Some(msg) = channel.wait().await {
        match msg {
            ChannelMsg::Data { ref data } => stdout.extend_from_slice(data),
            ChannelMsg::ExtendedData { ref data, .. } => stderr.extend_from_slice(data),
            ChannelMsg::ExitStatus { exit_status } => exit_code = exit_status,
            _ => {}
        }
    }

    let host_key_fp = state.lock().unwrap().seen_fp.clone().unwrap_or_default();

    Ok(ExecOutcome {
        result: SshResult {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            exit_code,
        },
        host_key_fp,
    })
}
