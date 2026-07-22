//! Liste des processus (gestionnaire de tâches) via SSH.

use serde::Serialize;

/// Top processus par CPU.
pub const LIST_CMD: &str =
    "ps -eo pid,user:20,pcpu,pmem,comm --sort=-pcpu --no-headers 2>/dev/null | head -80";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Process {
    pub pid: u32,
    pub user: String,
    pub cpu: f32,
    pub mem: f32,
    pub command: String,
}

pub fn parse(raw: &str) -> Vec<Process> {
    raw.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let pid = it.next()?.parse().ok()?;
            let user = it.next()?.to_string();
            let cpu = it.next()?.parse().unwrap_or(0.0);
            let mem = it.next()?.parse().unwrap_or(0.0);
            let command = it.collect::<Vec<_>>().join(" ");
            if command.is_empty() {
                return None;
            }
            Some(Process {
                pid,
                user,
                cpu,
                mem,
                command,
            })
        })
        .collect()
}
