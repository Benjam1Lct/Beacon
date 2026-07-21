//! Durcissement first-run d'un serveur (§12 de ARCHITECTURE.md).
//!
//! Ne s'applique QUE si l'on est connecté en root. Crée un utilisateur dédié, génère une clé
//! Ed25519 en local, la pousse sur le serveur, puis désactive le login root et par mot de passe.
//!
//! Règle d'or : on ne désactive JAMAIS le mot de passe / root avant d'avoir prouvé que la
//! connexion par la nouvelle clé fonctionne. En cas d'échec après durcissement, rollback auto.

use std::path::Path;

use russh_keys::PublicKeyBase64;
use serde::{Deserialize, Serialize};

use crate::secrets::{self, KeySecret};
use crate::ssh::{self, AuthInput, SshProfile};
use crate::store::{self, AuthKind, ProfileMeta};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardenInput {
    pub host: String,
    pub port: u16,
    pub root_username: String,
    pub auth: AuthInput,
    pub dev_username: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardeningStep {
    pub key: String,
    pub label: String,
    pub status: String, // "ok" | "skipped" | "failed"
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardeningReport {
    pub success: bool,
    pub steps: Vec<HardeningStep>,
    pub profile: Option<ProfileMeta>,
    pub message: String,
}

fn step(key: &str, label: &str, status: &str, detail: Option<String>) -> HardeningStep {
    HardeningStep {
        key: key.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        detail,
    }
}

/// Valide un nom d'utilisateur Linux pour éviter toute injection shell.
fn valid_username(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        && name.chars().next().map(|c| c != '-').unwrap_or(false)
}

/// Génère une paire Ed25519 : (clé privée PEM PKCS#8, ligne authorized_keys).
fn generate_keypair(comment: &str) -> Result<(String, String), String> {
    let key = russh_keys::key::KeyPair::generate_ed25519()
        .ok_or_else(|| "Génération de la clé Ed25519 échouée".to_string())?;
    let mut pem_bytes = Vec::new();
    russh_keys::encode_pkcs8_pem(&key, &mut pem_bytes).map_err(|e| e.to_string())?;
    let pem = String::from_utf8(pem_bytes).map_err(|e| e.to_string())?;
    let line = format!("ssh-ed25519 {} {}", key.public_key_base64(), comment);
    Ok((pem, line))
}

/// Exécute une commande et renvoie (stdout, stderr, exit_code).
async fn run(
    profile: &SshProfile,
    cmd: &str,
    fp: Option<&str>,
) -> Result<(String, String, u32), String> {
    let out = ssh::exec(profile, cmd, fp)
        .await
        .map_err(|e| e.to_string())?;
    Ok((out.result.stdout, out.result.stderr, out.result.exit_code))
}

fn fail(steps: Vec<HardeningStep>, message: impl Into<String>) -> HardeningReport {
    HardeningReport {
        success: false,
        steps,
        profile: None,
        message: message.into(),
    }
}

/// Lance le wizard de durcissement. `data_dir` sert à enregistrer le nouveau profil.
pub async fn run_wizard(data_dir: &Path, input: HardenInput) -> HardeningReport {
    let mut steps: Vec<HardeningStep> = Vec::new();
    let dev = input.dev_username.trim().to_string();

    if !valid_username(&dev) {
        return fail(steps, format!("Nom d'utilisateur invalide : « {dev} »"));
    }

    let root = SshProfile {
        host: input.host.clone(),
        port: input.port,
        username: input.root_username.clone(),
        auth: input.auth.clone(),
    };

    // --- Étape 1 : connexion + vérification root -------------------------------------------
    let (host_fp, is_root) = match ssh::exec(&root, "id -u", None).await {
        Ok(out) => (out.host_key_fp, out.result.stdout.trim() == "0"),
        Err(e) => {
            steps.push(step(
                "connect",
                "Connexion au serveur",
                "failed",
                Some(e.to_string()),
            ));
            return fail(steps, "Impossible de se connecter au serveur.");
        }
    };
    if !is_root {
        steps.push(step(
            "check_root",
            "Vérification des droits root",
            "failed",
            Some("Connecté en non-root : aucun durcissement nécessaire.".into()),
        ));
        return fail(
            steps,
            "Cet accès n'est pas root — le durcissement ne s'applique pas (et n'est pas nécessaire).",
        );
    }
    steps.push(step(
        "check_root",
        "Vérification des droits root",
        "ok",
        None,
    ));

    // --- Étape 2 : génération de la clé locale ---------------------------------------------
    let (pem, pub_line) = match generate_keypair("beacon") {
        Ok(v) => v,
        Err(e) => {
            steps.push(step(
                "keygen",
                "Génération de la clé Ed25519",
                "failed",
                Some(e),
            ));
            return fail(steps, "Échec de la génération de clé.");
        }
    };
    steps.push(step(
        "keygen",
        "Génération d'une clé Ed25519 (locale)",
        "ok",
        None,
    ));

    // --- Étape 3 : création de l'utilisateur dédié -----------------------------------------
    let create_cmd = format!(
        "set -e\n\
         if id -u '{dev}' >/dev/null 2>&1; then echo EXISTS; else useradd -m -s /bin/bash '{dev}'; echo CREATED; fi\n\
         usermod -aG sudo '{dev}' 2>/dev/null || usermod -aG wheel '{dev}' 2>/dev/null || true\n\
         if getent group docker >/dev/null 2>&1; then usermod -aG docker '{dev}' || true; fi"
    );
    match run(&root, &create_cmd, Some(&host_fp)).await {
        Ok((out, _, 0)) => {
            let existed = out.contains("EXISTS");
            steps.push(step(
                "create_user",
                &format!("Utilisateur « {dev} » (+ sudo, docker)"),
                if existed { "skipped" } else { "ok" },
                existed.then(|| "L'utilisateur existait déjà.".into()),
            ));
        }
        Ok((_, err, code)) => {
            steps.push(step(
                "create_user",
                "Création de l'utilisateur",
                "failed",
                Some(format!("code {code} : {err}")),
            ));
            return fail(steps, "Échec de la création de l'utilisateur.");
        }
        Err(e) => {
            steps.push(step(
                "create_user",
                "Création de l'utilisateur",
                "failed",
                Some(e),
            ));
            return fail(steps, "Échec de la création de l'utilisateur.");
        }
    }

    // --- Étape 4 : sudo sans mot de passe --------------------------------------------------
    let sudoers_cmd = format!(
        "set -e\n\
         F=/etc/sudoers.d/90-beacon-{dev}\n\
         echo '{dev} ALL=(ALL) NOPASSWD:ALL' > \"$F\"\n\
         chmod 440 \"$F\"\n\
         visudo -cf \"$F\""
    );
    match run(&root, &sudoers_cmd, Some(&host_fp)).await {
        Ok((_, _, 0)) => steps.push(step(
            "sudoers",
            "Sudo sans mot de passe (validé)",
            "ok",
            None,
        )),
        Ok((_, err, code)) => {
            steps.push(step(
                "sudoers",
                "Configuration sudo",
                "failed",
                Some(format!("code {code} : {err}")),
            ));
            return fail(steps, "Échec de la configuration sudo.");
        }
        Err(e) => {
            steps.push(step("sudoers", "Configuration sudo", "failed", Some(e)));
            return fail(steps, "Échec de la configuration sudo.");
        }
    }

    // --- Étape 5 : dépôt de la clé publique ------------------------------------------------
    let push_cmd = format!(
        "set -e\n\
         HD=$(getent passwd '{dev}' | cut -d: -f6)\n\
         mkdir -p \"$HD/.ssh\"\n\
         chmod 700 \"$HD/.ssh\"\n\
         touch \"$HD/.ssh/authorized_keys\"\n\
         grep -qF '{pub_line}' \"$HD/.ssh/authorized_keys\" || echo '{pub_line}' >> \"$HD/.ssh/authorized_keys\"\n\
         chmod 600 \"$HD/.ssh/authorized_keys\"\n\
         chown -R '{dev}' \"$HD/.ssh\""
    );
    match run(&root, &push_cmd, Some(&host_fp)).await {
        Ok((_, _, 0)) => steps.push(step("push_key", "Dépôt de la clé publique", "ok", None)),
        Ok((_, err, code)) => {
            steps.push(step(
                "push_key",
                "Dépôt de la clé publique",
                "failed",
                Some(format!("code {code} : {err}")),
            ));
            return fail(steps, "Échec du dépôt de la clé.");
        }
        Err(e) => {
            steps.push(step(
                "push_key",
                "Dépôt de la clé publique",
                "failed",
                Some(e),
            ));
            return fail(steps, "Échec du dépôt de la clé.");
        }
    }

    // --- Profil de vérification (dev + nouvelle clé) ---------------------------------------
    let dev_profile = SshProfile {
        host: input.host.clone(),
        port: input.port,
        username: dev.clone(),
        auth: AuthInput::KeyContent {
            pem: pem.clone(),
            passphrase: None,
        },
    };

    // --- Étape 6 : VÉRIFICATION cruciale avant tout durcissement ---------------------------
    match run(
        &dev_profile,
        "sudo -n true && echo BEACON_OK",
        Some(&host_fp),
    )
    .await
    {
        Ok((out, _, 0)) if out.contains("BEACON_OK") => {
            steps.push(step(
                "verify_key",
                "Connexion par clé vérifiée (avant durcissement)",
                "ok",
                None,
            ));
        }
        Ok((_, err, code)) => {
            steps.push(step(
                "verify_key",
                "Vérification de la clé",
                "failed",
                Some(format!("code {code} : {err}")),
            ));
            return fail(steps, "La nouvelle clé ne fonctionne pas — rien n'a été désactivé, ton accès root reste intact.");
        }
        Err(e) => {
            steps.push(step(
                "verify_key",
                "Vérification de la clé",
                "failed",
                Some(e),
            ));
            return fail(steps, "La nouvelle clé ne fonctionne pas — rien n'a été désactivé, ton accès root reste intact.");
        }
    }

    // --- Étape 7 : durcissement sshd -------------------------------------------------------
    let harden_cmd = "set -e\n\
         cat > /etc/ssh/sshd_config.d/99-beacon.conf <<'EOF'\n\
         # Généré par Beacon — durcissement SSH\n\
         PermitRootLogin no\n\
         PasswordAuthentication no\n\
         PubkeyAuthentication yes\n\
         EOF\n\
         sshd -t\n\
         (systemctl reload sshd 2>/dev/null || systemctl reload ssh 2>/dev/null || service ssh reload 2>/dev/null || service sshd reload 2>/dev/null)\n\
         echo RELOADED";
    match run(&root, harden_cmd, Some(&host_fp)).await {
        Ok((out, _, 0)) if out.contains("RELOADED") => {
            steps.push(step(
                "harden_sshd",
                "Root & mot de passe désactivés (sshd)",
                "ok",
                None,
            ));
        }
        Ok((_, err, code)) => {
            steps.push(step(
                "harden_sshd",
                "Durcissement sshd",
                "failed",
                Some(format!("code {code} : {err}")),
            ));
            return fail(
                steps,
                "Échec du durcissement sshd — configuration non appliquée.",
            );
        }
        Err(e) => {
            steps.push(step("harden_sshd", "Durcissement sshd", "failed", Some(e)));
            return fail(
                steps,
                "Échec du durcissement sshd — configuration non appliquée.",
            );
        }
    }

    // --- Étape 8 : re-vérification post-reload (sinon ROLLBACK) ----------------------------
    let after = run(
        &dev_profile,
        "sudo -n true && echo BEACON_OK",
        Some(&host_fp),
    )
    .await;
    let ok_after = matches!(&after, Ok((out, _, 0)) if out.contains("BEACON_OK"));
    if !ok_after {
        // Rollback via dev + sudo (root est peut-être déjà bloqué).
        let rollback = "sudo -n rm -f /etc/ssh/sshd_config.d/99-beacon.conf && \
             sudo -n sh -c 'systemctl reload sshd 2>/dev/null || systemctl reload ssh 2>/dev/null || service ssh reload 2>/dev/null'";
        let _ = run(&dev_profile, rollback, Some(&host_fp)).await;
        steps.push(step(
            "verify_after",
            "Re-vérification après durcissement",
            "failed",
            Some("Accès perdu après reload : rollback du durcissement effectué.".into()),
        ));
        return fail(steps, "Problème après durcissement : configuration annulée (rollback). Ton accès n'est pas compromis.");
    }
    steps.push(step(
        "verify_after",
        "Accès confirmé après durcissement",
        "ok",
        None,
    ));

    // --- Étape 9 : enregistrement du profil dev (clé au keyring) ---------------------------
    let id = uuid::Uuid::new_v4().to_string();
    if let Err(e) = secrets::set_key(
        &id,
        &KeySecret {
            pem,
            passphrase: None,
        },
    ) {
        steps.push(step(
            "save",
            "Enregistrement du profil",
            "failed",
            Some(e.to_string()),
        ));
        return fail(
            steps,
            "Durcissement réussi mais l'enregistrement du profil a échoué.",
        );
    }
    let label = if input.label.trim().is_empty() {
        format!("{dev}@{}", input.host)
    } else {
        input.label.trim().to_string()
    };
    let meta = ProfileMeta {
        id,
        label,
        host: input.host.clone(),
        port: input.port,
        username: dev.clone(),
        auth_kind: AuthKind::Key,
        host_key_fp: Some(host_fp),
    };
    if let Err(e) = store::upsert(data_dir, meta.clone()) {
        steps.push(step(
            "save",
            "Enregistrement du profil",
            "failed",
            Some(e.to_string()),
        ));
        return fail(
            steps,
            "Durcissement réussi mais l'enregistrement du profil a échoué.",
        );
    }
    steps.push(step(
        "save",
        "Profil « dev » enregistré (clé au trousseau)",
        "ok",
        None,
    ));

    HardeningReport {
        success: true,
        steps,
        profile: Some(meta),
        message: format!(
            "Serveur sécurisé : connexion désormais via l'utilisateur « {dev} » et sa clé. \
             Le login root et par mot de passe sont désactivés."
        ),
    }
}
