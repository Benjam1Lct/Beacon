//! Stockage des secrets dans le trousseau natif de l'OS (Windows Credential Manager /
//! macOS Keychain / Secret Service Linux). 100 % local, jamais en clair sur disque.

const SERVICE: &str = "beacon-ssh";

/// Secret d'un profil à clé (clé privée + passphrase), stocké en JSON dans le keyring.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct KeySecret {
    pub pem: String,
    pub passphrase: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("Trousseau indisponible : {0}")]
    Keyring(String),
    #[error("Secret illisible : {0}")]
    Parse(String),
}

impl serde::Serialize for SecretError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

fn entry(id: &str) -> Result<keyring::Entry, SecretError> {
    keyring::Entry::new(SERVICE, id).map_err(|e| SecretError::Keyring(e.to_string()))
}

/// Enregistre (ou remplace) le secret d'un profil.
pub fn set(id: &str, value: &str) -> Result<(), SecretError> {
    entry(id)?
        .set_password(value)
        .map_err(|e| SecretError::Keyring(e.to_string()))
}

/// Récupère le secret d'un profil, ou `None` s'il n'existe pas.
pub fn get(id: &str) -> Result<Option<String>, SecretError> {
    match entry(id)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(SecretError::Keyring(e.to_string())),
    }
}

/// Supprime le secret d'un profil (ignore l'absence).
pub fn delete(id: &str) -> Result<(), SecretError> {
    match entry(id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(SecretError::Keyring(e.to_string())),
    }
}

/// Enregistre la clé d'un profil (sérialisée en JSON).
pub fn set_key(id: &str, key: &KeySecret) -> Result<(), SecretError> {
    let blob = serde_json::to_string(key).map_err(|e| SecretError::Parse(e.to_string()))?;
    set(id, &blob)
}

/// Récupère la clé d'un profil, ou `None`.
pub fn get_key(id: &str) -> Result<Option<KeySecret>, SecretError> {
    match get(id)? {
        Some(blob) => serde_json::from_str(&blob)
            .map(Some)
            .map_err(|e| SecretError::Parse(e.to_string())),
        None => Ok(None),
    }
}
