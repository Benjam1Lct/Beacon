//! Explorateur de fichiers distant (lecture seule) via SSH.

use base64::Engine as _;
use serde::Serialize;

const MAX_PREVIEW: u64 = 5 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirListing {
    pub path: String,
    pub entries: Vec<DirEntry>,
}

/// Construit la commande de listing d'un dossier (chemin échappé).
pub fn list_cmd(path: &str) -> String {
    let target = if path.is_empty() || path == "~" {
        "\"$HOME\"".to_string()
    } else {
        format!("'{}'", path.replace('\'', "'\\''"))
    };
    format!(
        "cd -- {target} 2>/dev/null || cd -- \"$HOME\"\n\
         echo '===PWD==='\n\
         pwd\n\
         echo '===LS==='\n\
         ls -Ap1 --group-directories-first 2>/dev/null"
    )
}

/// Parse la sortie de `list_cmd`.
pub fn parse(raw: &str) -> DirListing {
    let mut section = "";
    let mut path = String::new();
    let mut entries = Vec::new();
    for line in raw.lines() {
        match line.trim() {
            "===PWD===" => {
                section = "PWD";
                continue;
            }
            "===LS===" => {
                section = "LS";
                continue;
            }
            _ => {}
        }
        match section {
            "PWD" => {
                if path.is_empty() {
                    path = line.trim().to_string();
                }
            }
            "LS" => {
                let l = line.trim_end();
                if l.is_empty() {
                    continue;
                }
                let is_dir = l.ends_with('/');
                let name = l.trim_end_matches('/').to_string();
                if !name.is_empty() {
                    entries.push(DirEntry { name, is_dir });
                }
            }
            _ => {}
        }
    }
    DirListing { path, entries }
}

// ---- Aperçu de fichier -----------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub kind: String, // "text" | "image" | "binary"
    pub name: String,
    pub mime: String,
    pub content: String, // texte, ou base64 pour une image
    pub size: u64,
    pub truncated: bool,
}

fn image_mime(name: &str) -> Option<&'static str> {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        "bmp" => Some("image/bmp"),
        "ico" => Some("image/x-icon"),
        _ => None,
    }
}

/// Commande de lecture : taille + contenu (base64, tronqué à 5 Mo) pour transfert sûr.
pub fn read_cmd(path: &str) -> String {
    let q = format!("'{}'", path.replace('\'', "'\\''"));
    format!(
        "echo '===SIZE==='\n\
         stat -c %s -- {q} 2>/dev/null || echo 0\n\
         echo '===B64==='\n\
         head -c 5242880 -- {q} 2>/dev/null | base64"
    )
}

/// Parse la sortie de `read_cmd` et décide du type (image / texte / binaire).
pub fn parse_preview(raw: &str, name: &str) -> FilePreview {
    let mut section = "";
    let mut size: u64 = 0;
    let mut b64 = String::new();
    for line in raw.lines() {
        match line.trim() {
            "===SIZE===" => {
                section = "S";
                continue;
            }
            "===B64===" => {
                section = "B";
                continue;
            }
            _ => {}
        }
        match section {
            "S" => {
                if size == 0 {
                    size = line.trim().parse().unwrap_or(0);
                }
            }
            "B" => b64.push_str(line.trim()),
            _ => {}
        }
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.as_bytes())
        .unwrap_or_default();
    let truncated = size > MAX_PREVIEW;

    if let Some(mime) = image_mime(name) {
        let content = base64::engine::general_purpose::STANDARD.encode(&bytes);
        return FilePreview {
            kind: "image".into(),
            name: name.into(),
            mime: mime.into(),
            content,
            size,
            truncated,
        };
    }

    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if ext == "pdf" {
        let content = base64::engine::general_purpose::STANDARD.encode(&bytes);
        return FilePreview {
            kind: "pdf".into(),
            name: name.into(),
            mime: "application/pdf".into(),
            content,
            size,
            truncated,
        };
    }

    if !bytes.contains(&0) && std::str::from_utf8(&bytes).is_ok() {
        FilePreview {
            kind: "text".into(),
            name: name.into(),
            mime: "text/plain".into(),
            content: String::from_utf8_lossy(&bytes).into_owned(),
            size,
            truncated,
        }
    } else {
        FilePreview {
            kind: "binary".into(),
            name: name.into(),
            mime: "application/octet-stream".into(),
            content: String::new(),
            size,
            truncated,
        }
    }
}
