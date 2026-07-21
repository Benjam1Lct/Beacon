use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handler};
use russh::ChannelMsg;
use serde::{Deserialize, Serialize};

/// Profil de connexion envoyé par le frontend.
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

/// Méthode d'authentification choisie par l'utilisateur.
///
/// Le mot de passe n'est utilisé que pour le bootstrap (durcissement) et n'est jamais persisté.
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
}

/// Résultat d'une commande distante.
#[derive(Debug, Serialize)]
pub struct SshResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u32,
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
    #[error("Erreur SSH : {0}")]
    Protocol(String),
}

impl Serialize for SshError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

/// Handler minimal : accepte la clé d'hôte (le pinning TOFU viendra en M1.x).
struct ClientHandler;

#[async_trait::async_trait]
impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO(M1.x) : afficher l'empreinte à l'utilisateur et l'épingler (TOFU).
        Ok(true)
    }
}

/// Se connecte au serveur et exécute `command`, en renvoyant la sortie.
pub async fn exec(profile: &SshProfile, command: &str) -> Result<SshResult, SshError> {
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        ..Default::default()
    });

    let addr = (profile.host.as_str(), profile.port);

    let connect = client::connect(config, addr, ClientHandler);
    let mut session = match tokio::time::timeout(Duration::from_secs(15), connect).await {
        Err(_) => return Err(SshError::Timeout),
        Ok(Err(e)) => return Err(SshError::Connect(profile.host.clone(), e.to_string())),
        Ok(Ok(s)) => s,
    };

    // Authentification
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
    };

    if !authenticated {
        return Err(SshError::Auth);
    }

    // Ouverture d'un canal et exécution
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

    Ok(SshResult {
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        exit_code,
    })
}
