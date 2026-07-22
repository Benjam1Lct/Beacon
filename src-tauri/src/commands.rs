//! Commandes Tauri exposées au frontend.
//!
//! Stockage strictement local : métadonnées dans un JSON du dossier de données de l'app,
//! secrets (clé + passphrase) dans le keyring de l'OS.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::{AppHandle, Manager};

use crate::caddy::{self, CaddyInfo, CaddyRoute, RouteHealth};
use crate::docker::{self, DeployConfig, DockerStatus};
use crate::files::{self, DirListing, FilePreview};
use crate::hardening::{self, HardenInput, HardeningReport};
use crate::monitor::{self, Metrics};
use crate::procs::{self, Process};
use crate::secrets::{self, KeySecret};
use crate::ssh::{self, AuthInput, ExecOutcome, SshProfile};
use crate::store::{self, AuthKind, ProfileMeta};
use crate::terminal::{self, TermTarget};

pub fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

/// Construit un profil SSH prêt à l'emploi depuis un profil enregistré (clé au keyring,
/// ou mot de passe fourni pour la session). Renvoie aussi les métadonnées.
fn resolve_profile(
    dir: &Path,
    id: &str,
    password: Option<String>,
) -> Result<(SshProfile, ProfileMeta), String> {
    let meta = store::get(dir, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Profil introuvable".to_string())?;

    let auth = match meta.auth_kind {
        AuthKind::Key => {
            let secret = secrets::get_key(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Clé absente du trousseau".to_string())?;
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
    Ok((profile, meta))
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
            secrets::set_key(&id, &secret).map_err(|e| e.to_string())?;
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
    let (profile, meta) = resolve_profile(&dir, &id, password)?;

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

/// Récupère les métriques système d'un serveur enregistré (CPU/RAM/disque/réseau).
/// Appelé périodiquement par le dashboard.
#[tauri::command]
pub async fn fetch_metrics(
    app: AppHandle,
    id: String,
    password: Option<String>,
) -> Result<Metrics, String> {
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, monitor::METRICS_CMD, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    monitor::parse(&out.result.stdout)
}

/// Liste les processus du serveur (gestionnaire de tâches, top par CPU).
#[tauri::command]
pub async fn list_processes(
    app: AppHandle,
    id: String,
    password: Option<String>,
) -> Result<Vec<Process>, String> {
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, procs::LIST_CMD, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(procs::parse(&out.result.stdout))
}

/// Liste un dossier distant (explorateur de fichiers, lecture seule).
#[tauri::command]
pub async fn list_dir(
    app: AppHandle,
    id: String,
    path: String,
    password: Option<String>,
) -> Result<DirListing, String> {
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(
        &profile,
        &files::list_cmd(&path),
        meta.host_key_fp.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(files::parse(&out.result.stdout))
}

/// Lit un fichier distant pour aperçu (texte / image / binaire).
#[tauri::command]
pub async fn read_file(
    app: AppHandle,
    id: String,
    path: String,
    password: Option<String>,
) -> Result<FilePreview, String> {
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(
        &profile,
        &files::read_cmd(&path),
        meta.host_key_fp.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;
    let name = path.rsplit('/').next().unwrap_or(&path).to_string();
    Ok(files::parse_preview(&out.result.stdout, &name))
}

/// Détecte Caddy (système ou conteneur Docker) et renvoie comment il tourne.
#[tauri::command]
pub async fn caddy_status(
    app: AppHandle,
    id: String,
    password: Option<String>,
) -> Result<CaddyInfo, String> {
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, caddy::STATUS_CMD, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(caddy::parse_info(&out.result.stdout))
}

/// Installe Caddy (dépôt officiel).
#[tauri::command]
pub async fn install_caddy(
    app: AppHandle,
    id: String,
    password: Option<String>,
) -> Result<String, String> {
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, caddy::INSTALL_CMD, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    if out.result.exit_code == 0 {
        Ok(out.result.stdout.trim().to_string())
    } else {
        Err(out.result.stderr.trim().to_string())
    }
}

/// Applique les liaisons reverse proxy : génère le Caddyfile, valide et recharge Caddy.
#[tauri::command]
pub async fn apply_routes(
    app: AppHandle,
    id: String,
    routes: Vec<CaddyRoute>,
    password: Option<String>,
) -> Result<(), String> {
    let caddyfile = caddy::generate(&routes);
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;

    // Détecte le mode (système / docker) pour appliquer au bon endroit.
    let info_out = ssh::exec(&profile, caddy::STATUS_CMD, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    let info = caddy::parse_info(&info_out.result.stdout);
    let cmd = caddy::apply_cmd(&caddyfile, &info)?;

    let out = ssh::exec(&profile, &cmd, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    if out.result.exit_code == 0 {
        Ok(())
    } else {
        Err(out.result.stderr.trim().to_string())
    }
}

/// Diagnostic des liaisons (DNS pointe vers le serveur ? service en écoute ?).
#[tauri::command]
pub async fn check_routes(
    app: AppHandle,
    id: String,
    routes: Vec<CaddyRoute>,
    password: Option<String>,
) -> Result<Vec<RouteHealth>, String> {
    if routes.is_empty() {
        return Ok(Vec::new());
    }
    let cmd = caddy::health_cmd(&routes);
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, &cmd, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(caddy::parse_health(&out.result.stdout))
}

/// Ouvre un terminal système avec une session SSH pré-remplie vers le serveur.
/// La clé (keyring) est exportée en fichier temporaire éphémère puis supprimée.
#[tauri::command]
pub fn open_ssh_terminal(app: AppHandle, id: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let meta = store::get(&dir, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Profil introuvable".to_string())?;
    let key_pem = match meta.auth_kind {
        AuthKind::Key => Some(
            secrets::get_key(&id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Clé absente du trousseau".to_string())?
                .pem,
        ),
        AuthKind::Password => None,
    };
    terminal::open(TermTarget {
        host: meta.host,
        port: meta.port,
        user: meta.username,
        key_pem,
        label: meta.label,
    })
}

/// Liste les conteneurs Docker d'un serveur (état + stats), et indique si Docker est installé.
#[tauri::command]
pub async fn docker_list(
    app: AppHandle,
    id: String,
    password: Option<String>,
) -> Result<DockerStatus, String> {
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, docker::LIST_CMD, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(docker::parse(&out.result.stdout))
}

/// Exécute une action Docker (start/stop/restart) sur un conteneur.
#[tauri::command]
pub async fn docker_action(
    app: AppHandle,
    id: String,
    container: String,
    action: String,
    password: Option<String>,
) -> Result<(), String> {
    let cmd =
        docker::action_cmd(&action, &container).ok_or_else(|| "Action invalide".to_string())?;
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, &cmd, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    if out.result.exit_code == 0 {
        Ok(())
    } else {
        Err(out.result.stderr.trim().to_string())
    }
}

/// Déploie une app du catalogue : construit et lance un `docker run` sur le serveur.
/// Renvoie l'ID du conteneur créé.
#[tauri::command]
pub async fn deploy_app(
    app: AppHandle,
    id: String,
    config: DeployConfig,
    password: Option<String>,
) -> Result<String, String> {
    let cmd = docker::run_cmd(&config).ok_or_else(|| "Configuration invalide".to_string())?;
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, &cmd, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    if out.result.exit_code == 0 {
        Ok(out.result.stdout.trim().to_string())
    } else {
        Err(out.result.stderr.trim().to_string())
    }
}

/// Installe Docker sur le serveur (script officiel get.docker.com).
#[tauri::command]
pub async fn install_docker(
    app: AppHandle,
    id: String,
    password: Option<String>,
) -> Result<String, String> {
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, docker::INSTALL_CMD, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    if out.result.exit_code == 0 {
        Ok(out.result.stdout.trim().to_string())
    } else {
        Err(out.result.stderr.trim().to_string())
    }
}

/// Renvoie les derniers logs d'un conteneur.
#[tauri::command]
pub async fn docker_logs(
    app: AppHandle,
    id: String,
    container: String,
    tail: u32,
    password: Option<String>,
) -> Result<String, String> {
    let cmd = docker::logs_cmd(&container, tail).ok_or_else(|| "Conteneur invalide".to_string())?;
    let dir = data_dir(&app)?;
    let (profile, meta) = resolve_profile(&dir, &id, password)?;
    let out = ssh::exec(&profile, &cmd, meta.host_key_fp.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(out.result.stdout)
}

/// Durcissement first-run : crée un user dédié, génère une clé, désactive root/mot de passe.
/// Ne s'applique que si l'accès fourni est root (garde-fou côté backend).
#[tauri::command]
pub async fn harden_bootstrap(
    app: AppHandle,
    input: HardenInput,
) -> Result<HardeningReport, String> {
    let dir = data_dir(&app)?;
    Ok(hardening::run_wizard(&dir, input).await)
}
