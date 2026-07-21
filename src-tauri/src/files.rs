//! Explorateur de fichiers distant (lecture seule) via SSH.

use serde::Serialize;

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
