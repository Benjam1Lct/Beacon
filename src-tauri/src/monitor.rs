//! Récupération et parsing des métriques système via SSH (M2).
//!
//! Une seule commande distante produit toutes les données (sections `===X===`),
//! parsées ici en une structure `Metrics` envoyée au dashboard.

use serde::Serialize;

/// Commande distante : imprime chaque métrique dans une section balisée.
/// Deux échantillons `/proc/stat` (à 1 s d'intervalle) permettent un calcul du % CPU stable.
pub const METRICS_CMD: &str = "LC_ALL=C\n\
     echo '===HOST==='; hostname 2>/dev/null\n\
     echo '===CORES==='; grep -c '^cpu[0-9]' /proc/stat\n\
     echo '===UPTIME==='; cat /proc/uptime\n\
     echo '===LOAD==='; cat /proc/loadavg\n\
     echo '===MEM==='; cat /proc/meminfo\n\
     echo '===CPU1==='; grep '^cpu ' /proc/stat\n\
     sleep 1\n\
     echo '===CPU2==='; grep '^cpu ' /proc/stat\n\
     echo '===DISK==='; df -kP /\n\
     echo '===NET==='; cat /proc/net/dev";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub hostname: String,
    pub uptime_secs: u64,
    pub cpu_cores: u32,
    pub cpu_percent: f32,
    pub load_avg: [f32; 3],
    pub mem_total_kb: u64,
    pub mem_available_kb: u64,
    pub disk_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
}

/// CPU cumulés d'une ligne `/proc/stat` : (total, idle).
fn cpu_totals(line: &str) -> Option<(u64, u64)> {
    let mut it = line.split_whitespace();
    if it.next()? != "cpu" {
        return None;
    }
    let vals: Vec<u64> = it.filter_map(|v| v.parse().ok()).collect();
    if vals.len() < 5 {
        return None;
    }
    let idle = vals[3] + vals.get(4).copied().unwrap_or(0); // idle + iowait
    let total: u64 = vals.iter().sum();
    Some((total, idle))
}

/// Parse la sortie brute de `METRICS_CMD`.
pub fn parse(raw: &str) -> Result<Metrics, String> {
    // Regroupe les lignes par section.
    let mut section = "";
    let mut hostname = String::new();
    let mut cores: u32 = 1;
    let mut uptime_secs: u64 = 0;
    let mut load_avg = [0.0f32; 3];
    let (mut mem_total_kb, mut mem_available_kb) = (0u64, 0u64);
    let (mut disk_total_bytes, mut disk_used_bytes) = (0u64, 0u64);
    let (mut net_rx_bytes, mut net_tx_bytes) = (0u64, 0u64);
    let mut cpu1: Option<(u64, u64)> = None;
    let mut cpu2: Option<(u64, u64)> = None;

    for line in raw.lines() {
        let l = line.trim();
        if let Some(name) = l.strip_prefix("===").and_then(|s| s.strip_suffix("===")) {
            section = match name {
                "HOST" => "HOST",
                "CORES" => "CORES",
                "UPTIME" => "UPTIME",
                "LOAD" => "LOAD",
                "MEM" => "MEM",
                "CPU1" => "CPU1",
                "CPU2" => "CPU2",
                "DISK" => "DISK",
                "NET" => "NET",
                _ => "",
            };
            continue;
        }
        if l.is_empty() {
            continue;
        }
        match section {
            "HOST" => hostname = l.to_string(),
            "CORES" => cores = l.parse().unwrap_or(1).max(1),
            "UPTIME" => {
                uptime_secs = l
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|f| f as u64)
                    .unwrap_or(0);
            }
            "LOAD" => {
                let parts: Vec<f32> = l
                    .split_whitespace()
                    .take(3)
                    .filter_map(|s| s.parse().ok())
                    .collect();
                for (i, v) in parts.into_iter().enumerate() {
                    load_avg[i] = v;
                }
            }
            "MEM" => {
                if let Some(rest) = l.strip_prefix("MemTotal:") {
                    mem_total_kb = rest
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                } else if let Some(rest) = l.strip_prefix("MemAvailable:") {
                    mem_available_kb = rest
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                }
            }
            "CPU1" => cpu1 = cpu_totals(l),
            "CPU2" => cpu2 = cpu_totals(l),
            "DISK" => {
                // On ignore l'en-tête (commence par "Filesystem").
                if !l.starts_with("Filesystem") {
                    let f: Vec<&str> = l.split_whitespace().collect();
                    if f.len() >= 3 {
                        disk_total_bytes = f[1].parse::<u64>().unwrap_or(0) * 1024;
                        disk_used_bytes = f[2].parse::<u64>().unwrap_or(0) * 1024;
                    }
                }
            }
            "NET" => {
                // Format: "iface: rx_bytes ... tx_bytes ...", on ignore lo et les en-têtes.
                if let Some((iface, stats)) = l.split_once(':') {
                    if iface.trim() != "lo" {
                        let nums: Vec<u64> = stats
                            .split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        if nums.len() >= 9 {
                            net_rx_bytes += nums[0];
                            net_tx_bytes += nums[8];
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let cpu_percent = match (cpu1, cpu2) {
        (Some((t1, i1)), Some((t2, i2))) if t2 > t1 => {
            let dt = (t2 - t1) as f32;
            let di = (i2.saturating_sub(i1)) as f32;
            (100.0 * (dt - di) / dt).clamp(0.0, 100.0)
        }
        _ => 0.0,
    };

    if mem_total_kb == 0 {
        return Err("Métriques illisibles (mémoire introuvable).".into());
    }

    Ok(Metrics {
        hostname,
        uptime_secs,
        cpu_cores: cores,
        cpu_percent,
        load_avg,
        mem_total_kb,
        mem_available_kb,
        disk_total_bytes,
        disk_used_bytes,
        net_rx_bytes,
        net_tx_bytes,
    })
}
