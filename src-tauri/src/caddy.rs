//! Reverse proxy & SSL via Caddy (M5).
//!
//! Génère un Caddyfile à partir des liaisons (domaine -> conteneur:port), l'applique sur le
//! serveur et recharge Caddy. Gère le SSL public (Let's Encrypt) et local (cert interne).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaddyRoute {
    pub domain: String,
    pub target_port: u16,
    /// "public" (Let's Encrypt) | "local" (tls internal) | "none" (http)
    pub ssl: String,
}

fn valid_domain(d: &str) -> bool {
    !d.is_empty()
        && d.len() <= 253
        && d.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        && !d.starts_with('-')
        && d.contains(|c: char| c.is_ascii_alphabetic() || c == '.')
}

/// Génère le Caddyfile à partir des liaisons valides.
pub fn generate(routes: &[CaddyRoute]) -> String {
    let mut out = String::from("# Généré par Beacon — ne pas éditer à la main\n");
    for r in routes {
        if !valid_domain(&r.domain) {
            continue;
        }
        let block = match r.ssl.as_str() {
            "local" => format!(
                "{d} {{\n\ttls internal\n\treverse_proxy 127.0.0.1:{p}\n}}\n",
                d = r.domain,
                p = r.target_port
            ),
            "none" => format!(
                "http://{d} {{\n\treverse_proxy 127.0.0.1:{p}\n}}\n",
                d = r.domain,
                p = r.target_port
            ),
            _ => format!(
                "{d} {{\n\treverse_proxy 127.0.0.1:{p}\n}}\n",
                d = r.domain,
                p = r.target_port
            ),
        };
        out.push_str(&block);
    }
    out
}

/// Où et comment Caddy tourne sur le serveur.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaddyInfo {
    pub installed: bool,
    pub mode: String, // "system" | "docker" | "none"
    pub container: Option<String>,
    /// Chemin hôte du Caddyfile (pour écrire) — mode docker.
    pub config_src: Option<String>,
    /// Chemin du Caddyfile dans le conteneur (pour recharger) — mode docker.
    pub config_dst: Option<String>,
}

/// Détection : Caddy système, ou conteneur Docker dont l'image contient "caddy".
pub const STATUS_CMD: &str = "if command -v caddy >/dev/null 2>&1 || [ -x /usr/bin/caddy ] \
     || [ -x /usr/local/bin/caddy ] || systemctl list-unit-files 2>/dev/null | grep -q '^caddy'; then\n\
       echo 'MODE system'\n\
     elif command -v docker >/dev/null 2>&1 && CONT=$(docker ps --format '{{.Names}};{{.Image}}' 2>/dev/null | grep -i caddy | head -1 | cut -d';' -f1) && [ -n \"$CONT\" ]; then\n\
       echo 'MODE docker'\n\
       echo \"CONTAINER $CONT\"\n\
       echo '===MOUNTS==='\n\
       docker inspect \"$CONT\" --format '{{range .Mounts}}{{.Source}};{{.Destination}}{{\"\\n\"}}{{end}}' 2>/dev/null\n\
     else\n\
       echo 'MODE none'\n\
     fi";

/// Parse la sortie de `STATUS_CMD`.
pub fn parse_info(raw: &str) -> CaddyInfo {
    let mut mode = "none".to_string();
    let mut container = None;
    let mut mounts: Vec<(String, String)> = Vec::new();
    let mut in_mounts = false;

    for line in raw.lines() {
        let l = line.trim();
        if let Some(m) = l.strip_prefix("MODE ") {
            mode = m.trim().to_string();
        } else if let Some(c) = l.strip_prefix("CONTAINER ") {
            container = Some(c.trim().to_string());
        } else if l == "===MOUNTS===" {
            in_mounts = true;
        } else if in_mounts && !l.is_empty() {
            if let Some((src, dst)) = l.split_once(';') {
                mounts.push((src.to_string(), dst.to_string()));
            }
        }
    }

    // Cherche le Caddyfile parmi les montages.
    let (mut config_src, mut config_dst) = (None, None);
    for (src, dst) in &mounts {
        if dst.rsplit('/').next() == Some("Caddyfile") {
            config_src = Some(src.clone());
            config_dst = Some(dst.clone());
            break;
        }
    }
    if config_src.is_none() {
        for (src, dst) in &mounts {
            let base = dst.trim_end_matches('/').rsplit('/').next().unwrap_or("");
            if dst == "/etc/caddy" || base == "caddy" {
                config_src = Some(format!("{}/Caddyfile", src.trim_end_matches('/')));
                config_dst = Some(format!("{}/Caddyfile", dst.trim_end_matches('/')));
                break;
            }
        }
    }

    CaddyInfo {
        installed: mode != "none",
        mode,
        container,
        config_src,
        config_dst,
    }
}

fn quote(p: &str) -> String {
    format!("'{}'", p.replace('\'', "'\\''"))
}

/// Commande d'application selon le mode (système ou Docker).
pub fn apply_cmd(caddyfile: &str, info: &CaddyInfo) -> Result<String, String> {
    if info.mode == "docker" {
        let (src, dst, cont) = match (&info.config_src, &info.config_dst, &info.container) {
            (Some(s), Some(d), Some(c)) => (s, d, c),
            _ => {
                return Err("Caddy tourne dans Docker mais son Caddyfile n'est pas monté depuis l'hôte. Ajoute un volume (ex: -v /srv/caddy/Caddyfile:/etc/caddy/Caddyfile) pour que Beacon puisse le gérer.".into());
            }
        };
        Ok(format!(
            "set -e\n\
             if [ \"$(id -u)\" = 0 ]; then SUDO=\"\"; else SUDO=\"sudo -n\"; fi\n\
             cat <<'BEACON_CADDY_EOF' | $SUDO tee {src} >/dev/null\n\
             {caddyfile}\n\
             BEACON_CADDY_EOF\n\
             docker exec {cont} caddy reload --config {dst} --adapter caddyfile 2>/dev/null || docker restart {cont}\n\
             echo DONE",
            src = quote(src),
            dst = quote(dst),
            cont = quote(cont),
        ))
    } else {
        Ok(format!(
            "set -e\n\
             if [ \"$(id -u)\" = 0 ]; then SUDO=\"\"; else SUDO=\"sudo -n\"; fi\n\
             $SUDO mkdir -p /etc/caddy\n\
             cat <<'BEACON_CADDY_EOF' | $SUDO tee /etc/caddy/Caddyfile >/dev/null\n\
             {caddyfile}\n\
             BEACON_CADDY_EOF\n\
             $SUDO caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile\n\
             $SUDO systemctl reload caddy 2>/dev/null || $SUDO systemctl restart caddy\n\
             echo DONE"
        ))
    }
}

/// Détection + installation de Caddy (dépôt officiel, Debian/Ubuntu).
pub const INSTALL_CMD: &str = "set -e\n\
     if command -v caddy >/dev/null 2>&1; then echo ALREADY; exit 0; fi\n\
     if [ \"$(id -u)\" = 0 ]; then SUDO=\"\"; else SUDO=\"sudo -n\"; fi\n\
     $SUDO apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl gnupg\n\
     curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | $SUDO gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg\n\
     curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | $SUDO tee /etc/apt/sources.list.d/caddy-stable.list >/dev/null\n\
     $SUDO apt-get update\n\
     $SUDO apt-get install -y caddy\n\
     echo DONE";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteHealth {
    pub domain: String,
    pub dns_ok: bool,
    pub port_ok: bool,
    pub resolved_ip: String,
    pub server_ip: String,
}

/// Construit la commande de diagnostic (IP publique + DNS/port par domaine).
pub fn health_cmd(routes: &[CaddyRoute]) -> String {
    let mut c = String::from(
        "PUBIP=$(curl -s --max-time 4 https://api.ipify.org 2>/dev/null || curl -s --max-time 4 ifconfig.me 2>/dev/null)\n\
         echo \"PUBIP $PUBIP\"\n",
    );
    for r in routes {
        if !valid_domain(&r.domain) {
            continue;
        }
        c.push_str("echo '===R==='\n");
        c.push_str(&format!("echo \"DOMAIN {}\"\n", r.domain));
        c.push_str(&format!(
            "echo \"DNS $(getent hosts '{}' 2>/dev/null | awk '{{print $1}}' | head -1)\"\n",
            r.domain
        ));
        c.push_str(&format!(
            "curl -sf -o /dev/null --max-time 3 http://127.0.0.1:{p} && echo 'PORT ok' || echo 'PORT down'\n",
            p = r.target_port
        ));
    }
    c
}

/// Parse la sortie de `health_cmd`.
pub fn parse_health(raw: &str) -> Vec<RouteHealth> {
    let mut server_ip = String::new();
    let mut out = Vec::new();
    let mut cur: Option<RouteHealth> = None;

    for line in raw.lines() {
        let l = line.trim();
        if let Some(ip) = l.strip_prefix("PUBIP ") {
            server_ip = ip.trim().to_string();
        } else if l == "===R===" {
            if let Some(r) = cur.take() {
                out.push(r);
            }
            cur = Some(RouteHealth {
                domain: String::new(),
                dns_ok: false,
                port_ok: false,
                resolved_ip: String::new(),
                server_ip: server_ip.clone(),
            });
        } else if let Some(d) = l.strip_prefix("DOMAIN ") {
            if let Some(r) = cur.as_mut() {
                r.domain = d.trim().to_string();
            }
        } else if let Some(ip) = l.strip_prefix("DNS ") {
            if let Some(r) = cur.as_mut() {
                r.resolved_ip = ip.trim().to_string();
                r.dns_ok = !r.resolved_ip.is_empty() && r.resolved_ip == server_ip;
            }
        } else if l == "PORT ok" {
            if let Some(r) = cur.as_mut() {
                r.port_ok = true;
            }
        }
    }
    if let Some(r) = cur.take() {
        out.push(r);
    }
    out
}
