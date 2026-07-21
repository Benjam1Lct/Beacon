//! Ouvre un terminal système avec une session SSH pré-remplie vers un serveur.
//!
//! Pour l'auth par clé, la clé (au keyring) est écrite dans un fichier temporaire à usage
//! unique, passée à `ssh -i`, puis supprimée peu après (ssh la lit à la connexion).

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

pub struct TermTarget {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_pem: Option<String>,
    pub label: String,
}

fn valid_host(h: &str) -> bool {
    !h.is_empty()
        && h.len() <= 255
        && h.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
}

fn valid_user(u: &str) -> bool {
    !u.is_empty()
        && u.len() <= 64
        && u.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// Écrit la clé dans un fichier temporaire (perms restreintes sur Unix).
fn write_temp_key(pem: &str) -> Result<PathBuf, String> {
    let mut p = std::env::temp_dir();
    p.push(format!("beacon-ssh-{}.key", uuid::Uuid::new_v4()));
    std::fs::write(&p, pem).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
    }
    Ok(p)
}

/// Programme la suppression du fichier après un délai (backstop de nettoyage).
fn schedule_delete(path: PathBuf) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(60));
        let _ = std::fs::remove_file(path);
    });
}

/// Ouvre un terminal avec la session SSH.
pub fn open(t: TermTarget) -> Result<(), String> {
    if !valid_host(&t.host) {
        return Err("Hôte invalide".into());
    }
    if !valid_user(&t.user) {
        return Err("Utilisateur invalide".into());
    }

    let key_path = match &t.key_pem {
        Some(pem) => Some(write_temp_key(pem)?),
        None => None,
    };

    let res = launch(&t, key_path.as_ref());

    if let Some(p) = key_path {
        schedule_delete(p);
    }
    res
}

#[cfg(windows)]
fn launch(t: &TermTarget, key: Option<&PathBuf>) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let mut ssh = String::from("ssh -o StrictHostKeyChecking=accept-new");
    ssh.push_str(&format!(" -p {}", t.port));
    if let Some(k) = key {
        ssh.push_str(&format!(" -i \"{}\"", k.display()));
    }
    ssh.push_str(&format!(" {}@{}", t.user, t.host));

    // Batch temporaire : lance ssh, attend, puis se supprime avec la clé.
    let mut bat = std::env::temp_dir();
    bat.push(format!("beacon-ssh-{}.bat", uuid::Uuid::new_v4()));
    let label = t.label.replace(['\r', '\n', '%'], "");
    let del_key = key
        .map(|k| format!("del \"{}\" >nul 2>&1\r\n", k.display()))
        .unwrap_or_default();
    let content = format!(
        "@echo off\r\ntitle Beacon SSH - {label}\r\n{ssh}\r\necho.\r\necho [Session SSH terminee]\r\npause >nul\r\n{del_key}(goto) 2>nul & del \"%~f0\"\r\n"
    );
    std::fs::write(&bat, content).map_err(|e| e.to_string())?;

    Command::new("cmd")
        .raw_arg(format!("/C start \"Beacon SSH\" \"{}\"", bat.display()))
        .spawn()
        .map_err(|e| format!("Impossible d'ouvrir le terminal : {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn launch(t: &TermTarget, key: Option<&PathBuf>) -> Result<(), String> {
    let key_flag = key
        .map(|k| format!("-i {} ", shell_quote(&k.display().to_string())))
        .unwrap_or_default();
    let ssh = format!(
        "ssh -o StrictHostKeyChecking=accept-new -p {} {}{}@{}",
        t.port, key_flag, t.user, t.host
    );
    let script = format!(
        "tell application \"Terminal\" to do script \"{}\"",
        ssh.replace('"', "\\\"")
    );
    Command::new("osascript")
        .arg("-e")
        .arg(script)
        .spawn()
        .map_err(|e| format!("Impossible d'ouvrir le terminal : {e}"))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn launch(t: &TermTarget, key: Option<&PathBuf>) -> Result<(), String> {
    let key_flag = key
        .map(|k| format!("-i {} ", shell_quote(&k.display().to_string())))
        .unwrap_or_default();
    let ssh = format!(
        "ssh -o StrictHostKeyChecking=accept-new -p {} {}{}@{}",
        t.port, key_flag, t.user, t.host
    );
    let inner = format!("{ssh}; echo; echo '[Session SSH terminee]'; exec bash");
    for term in [
        "x-terminal-emulator",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "xterm",
    ] {
        if Command::new(term)
            .arg("-e")
            .arg(format!("bash -c \"{}\"", inner.replace('"', "\\\"")))
            .spawn()
            .is_ok()
        {
            return Ok(());
        }
    }
    Err("Aucun émulateur de terminal trouvé".into())
}

#[cfg(unix)]
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
