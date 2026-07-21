//! Commandes Tauri exposées au frontend.
//!
//! Stockage strictement local : métadonnées dans un JSON du dossier de données de l'app,
//! secrets (clé + passphrase) dans le keyring de l'OS.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::secrets;
use crate::ssh::{self, AuthInput, ExecOutcome, SshProfile};
use crate::store::{self, AuthKind, ProfileMeta};

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

/// Secret d'un profil à clé, stocké tel quel dans le keyring.
#[derive(Serialize, Deserialize)]
struct KeySecret {
    pem: String,
    passphrase: Option<String>,
}

/// Teste une connexion ad-hoc (flux d'import, avant enregistrement).
/// Renvoie la sortie de `uname -a` et l'empreinte de la clé d'hôte observée.
#[tauri::command]
pub async fn ssh_test_connection(profile: SshProfile) -> Result<ExecOutcome, String> {
    ssh::exec(&profile, "uname -a", None)
        .await
        .map_err(|e| e.to_string())
}

/// Liste les profils enregistrés (métadonnées, sans secret).
#[tauri::command]
pub fn list_profiles(app: AppHandle) -> Result<Vec<ProfileMeta>, String> {
    store::load(&data_dir(&app)?).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProfileInput {
    label: String,
    host: String,
    port: u16,
    username: String,
    auth: SaveAuth,
    /// Empreinte de la clé d'hôte issue d'un test réussi, à épingler.
    host_key_fp: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SaveAuth {
    Key {
        path: String,
        passphrase: Option<String>,
    },
    Password,
}

/// Enregistre un profil. Pour une clé, son contenu est lu et rangé dans le keyring
/// (le fichier d'origine n'est plus nécessaire). Un mot de passe n'est jamais persisté.
#[tauri::command]
pub fn save_profile(app: AppHandle, input: SaveProfileInput) -> Result<ProfileMeta, String> {
    let dir = data_dir(&app)?;
    let id = uuid::Uuid::new_v4().to_string();

    let auth_kind = match &input.auth {
        SaveAuth::Key { path, passphrase } => {
            let pem = std::fs::read_to_string(path)
                .map_err(|e| format!("Clé illisible ({path}) : {e}"))?;
            let secret = KeySecret {
                pem,
                passphrase: passphrase.clone(),
            };
            let blob = serde_json::to_string(&secret).map_err(|e| e.to_string())?;
            secrets::set(&id, &blob).map_err(|e| e.to_string())?;
            AuthKind::Key
        }
        SaveAuth::Password => AuthKind::Password,
    };

    let meta = ProfileMeta {
        id,
        label: input.label,
        host: input.host,
        port: input.port,
        username: input.username,
        auth_kind,
        host_key_fp: input.host_key_fp,
    };

    store::upsert(&dir, meta.clone()).map_err(|e| e.to_string())?;
    Ok(meta)
}

/// Supprime un profil et son secret associé.
#[tauri::command]
pub fn delete_profile(app: AppHandle, id: String) -> Result<(), String> {
    secrets::delete(&id).map_err(|e| e.to_string())?;
    store::remove(&data_dir(&app)?, &id).map_err(|e| e.to_string())
}

/// Se connecte à un profil enregistré, applique le pinning TOFU et renvoie `uname -a`.
/// Au premier contact, l'empreinte de la clé d'hôte est épinglée automatiquement.
/// Pour un profil par mot de passe, `password` doit être fourni (jamais stocké).
#[tauri::command]
pub async fn connect_profile(
    app: AppHandle,
    id: String,
    password: Option<String>,
) -> Result<ExecOutcome, String> {
    let dir = data_dir(&app)?;
    let meta = store::get(&dir, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Profil introuvable".to_string())?;

    let auth = match meta.auth_kind {
        AuthKind::Key => {
            let blob = secrets::get(&id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Clé absente du trousseau".to_string())?;
            let secret: KeySecret = serde_json::from_str(&blob).map_err(|e| e.to_string())?;
            AuthInput::KeyContent {
                pem: secret.pem,
                passphrase: secret.passphrase,
            }
        }
        AuthKind::Password => {
            let password = password.ok_or_else(|| "Mot de passe requis".to_string())?;
            AuthInput::Password { password }
        }
    };

    let profile = SshProfile {
        host: meta.host.clone(),
        port: meta.port,
        username: meta.username.clone(),
        auth,
    };

    let outcome = ssh::exec(&profile, "uname -a", meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    // TOFU : au premier contact, on épingle l'empreinte observée.
    if meta.host_key_fp.is_none() {
        let mut updated = meta;
        updated.host_key_fp = Some(outcome.host_key_fp.clone());
        store::upsert(&dir, updated).map_err(|e| e.to_string())?;
    }

    Ok(outcome)
}
