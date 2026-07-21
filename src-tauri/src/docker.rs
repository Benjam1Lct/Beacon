//! Gestion Docker distante via SSH (M3).
//!
//! Détecte Docker, liste les conteneurs (`docker ps -a`) et fusionne les stats
//! (`docker stats`) ; expose start/stop/restart et les logs.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Détecte Docker puis liste conteneurs et stats en une commande.
pub const LIST_CMD: &str =
    "if ! command -v docker >/dev/null 2>&1; then echo __NO_DOCKER__; exit 0; fi\n\
     echo '===PS==='\n\
     docker ps -a --format '{{json .}}' 2>/dev/null\n\
     echo '===STATS==='\n\
     docker stats --no-stream --format '{{json .}}' 2>/dev/null";

#[derive(Deserialize)]
struct PsLine {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Names", default)]
    names: String,
    #[serde(rename = "Image", default)]
    image: String,
    #[serde(rename = "State", default)]
    state: String,
    #[serde(rename = "Status", default)]
    status: String,
    #[serde(rename = "Ports", default)]
    ports: String,
}

#[derive(Deserialize)]
struct StatLine {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "CPUPerc", default)]
    cpu: String,
    #[serde(rename = "MemUsage", default)]
    mem: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: String,
    pub cpu_percent: Option<f32>,
    pub mem_usage: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerStatus {
    pub installed: bool,
    pub containers: Vec<Container>,
}

/// Parse la sortie de `LIST_CMD`.
pub fn parse(raw: &str) -> DockerStatus {
    if raw.contains("__NO_DOCKER__") {
        return DockerStatus {
            installed: false,
            containers: Vec::new(),
        };
    }

    let mut section = "";
    let mut ps: Vec<PsLine> = Vec::new();
    let mut stats: Vec<StatLine> = Vec::new();

    for line in raw.lines() {
        let l = line.trim();
        match l {
            "===PS===" => {
                section = "PS";
                continue;
            }
            "===STATS===" => {
                section = "STATS";
                continue;
            }
            "" => continue,
            _ => {}
        }
        match section {
            "PS" => {
                if let Ok(p) = serde_json::from_str::<PsLine>(l) {
                    ps.push(p);
                }
            }
            "STATS" => {
                if let Ok(s) = serde_json::from_str::<StatLine>(l) {
                    stats.push(s);
                }
            }
            _ => {}
        }
    }

    // Index des stats par id court et par nom.
    let mut by_id: HashMap<String, &StatLine> = HashMap::new();
    let mut by_name: HashMap<String, &StatLine> = HashMap::new();
    for s in &stats {
        if !s.id.is_empty() {
            by_id.insert(s.id.clone(), s);
        }
        if !s.name.is_empty() {
            by_name.insert(s.name.clone(), s);
        }
    }

    let containers = ps
        .into_iter()
        .map(|p| {
            let stat = by_id.get(&p.id).or_else(|| by_name.get(&p.names)).copied();
            let cpu_percent =
                stat.and_then(|s| s.cpu.trim_end_matches('%').trim().parse::<f32>().ok());
            let mem_usage = stat.map(|s| s.mem.clone()).filter(|m| !m.is_empty());
            // State absent sur d'anciens Docker : on déduit du Status.
            let state = if p.state.is_empty() {
                if p.status.starts_with("Up") {
                    "running".to_string()
                } else {
                    "exited".to_string()
                }
            } else {
                p.state
            };
            Container {
                id: p.id,
                name: p.names,
                image: p.image,
                state,
                status: p.status,
                ports: p.ports,
                cpu_percent,
                mem_usage,
            }
        })
        .collect();

    DockerStatus {
        installed: true,
        containers,
    }
}

/// Valide un id/nom de conteneur (anti-injection).
pub fn valid_ref(r: &str) -> bool {
    !r.is_empty()
        && r.len() <= 128
        && r.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Construit la commande d'action (start/stop/restart) après validation.
pub fn action_cmd(action: &str, container: &str) -> Option<String> {
    if !valid_ref(container) {
        return None;
    }
    match action {
        "start" | "stop" | "restart" => Some(format!("docker {action} '{container}'")),
        _ => None,
    }
}

/// Commande pour lire les logs (tail borné).
pub fn logs_cmd(container: &str, tail: u32) -> Option<String> {
    if !valid_ref(container) {
        return None;
    }
    let tail = tail.clamp(1, 1000);
    Some(format!("docker logs --tail {tail} '{container}' 2>&1"))
}
