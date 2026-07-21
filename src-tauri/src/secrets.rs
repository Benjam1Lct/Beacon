//! Stockage des secrets dans le trousseau natif de l'OS (Windows Credential Manager /
//! macOS Keychain / Secret Service Linux). 100 % local, jamais en clair sur disque.

const SERVICE: &str = "beacon-ssh";

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("Trousseau indisponible : {0}")]
    Keyring(String),
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
