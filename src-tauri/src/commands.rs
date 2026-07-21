//! Commandes Tauri exposées au frontend.

use crate::ssh::{self, SshProfile, SshResult};

/// Teste la connexion SSH : se connecte et renvoie `uname -a`.
/// Utilisé par l'écran de connexion (M1) pour valider un profil.
#[tauri::command]
pub async fn ssh_test_connection(profile: SshProfile) -> Result<SshResult, ssh::SshError> {
    ssh::exec(&profile, "uname -a").await
}
