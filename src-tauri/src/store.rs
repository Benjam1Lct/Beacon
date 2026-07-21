//! Persistance locale des profils de serveurs (métadonnées non secrètes).
//!
//! Fichier JSON dans le dossier de données de l'app. Les secrets (clé, passphrase) vivent
//! dans le keyring (voir `secrets.rs`), jamais ici.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AuthKind {
    Key,
    Password,
}

/// Métadonnées d'un serveur enregistré. Aucun secret ici.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMeta {
    pub id: String,
    pub label: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_kind: AuthKind,
    /// Empreinte de la clé d'hôte épinglée au 1er contact (TOFU). `None` = pas encore vu.
    #[serde(default)]
    pub host_key_fp: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Accès au stockage impossible : {0}")]
    Io(String),
    #[error("Fichier de profils corrompu : {0}")]
    Parse(String),
}

impl serde::Serialize for StoreError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

fn profiles_path(dir: &Path) -> PathBuf {
    dir.join("profiles.json")
}

/// Charge la liste des profils (vide si le fichier n'existe pas encore).
pub fn load(dir: &Path) -> Result<Vec<ProfileMeta>, StoreError> {
    let path = profiles_path(dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| StoreError::Io(e.to_string()))?;
    serde_json::from_str(&raw).map_err(|e| StoreError::Parse(e.to_string()))
}

fn save_all(dir: &Path, profiles: &[ProfileMeta]) -> Result<(), StoreError> {
    std::fs::create_dir_all(dir).map_err(|e| StoreError::Io(e.to_string()))?;
    let raw =
        serde_json::to_string_pretty(profiles).map_err(|e| StoreError::Parse(e.to_string()))?;
    std::fs::write(profiles_path(dir), raw).map_err(|e| StoreError::Io(e.to_string()))
}

/// Récupère un profil par id.
pub fn get(dir: &Path, id: &str) -> Result<Option<ProfileMeta>, StoreError> {
    Ok(load(dir)?.into_iter().find(|p| p.id == id))
}

/// Insère ou met à jour un profil (par id).
pub fn upsert(dir: &Path, profile: ProfileMeta) -> Result<(), StoreError> {
    let mut profiles = load(dir)?;
    match profiles.iter_mut().find(|p| p.id == profile.id) {
        Some(existing) => *existing = profile,
        None => profiles.push(profile),
    }
    save_all(dir, &profiles)
}

/// Supprime un profil par id.
pub fn remove(dir: &Path, id: &str) -> Result<(), StoreError> {
    let mut profiles = load(dir)?;
    profiles.retain(|p| p.id != id);
    save_all(dir, &profiles)
}
