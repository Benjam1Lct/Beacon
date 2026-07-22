//! Reverse proxy & SSL via Caddy (M5).
//!
//! Génère un Caddyfile à partir des liaisons (domaine -> conteneur:port), l'applique sur le
//! serveur et recharge Caddy. Gère le SSL public (Let's Encrypt) et local (cert interne).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaddyRoute {
    pub domain: String,
    pub target_port: u16,
    /// "public" (Let's Encrypt) | "local" (tls internal) | "none" (http)
    pub ssl: String,
    /// true si le bloc est géré par Beacon (entre marqueurs) et donc supprimable.
    #[serde(default)]
    pub managed: bool,
}

/// Bloc site Caddyfile pour une liaison (sans marqueurs).
fn block_for(r: &CaddyRoute) -> String {
    match r.ssl.as_str() {
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
    }
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

/// Commande pour lire le Caddyfile courant selon le mode détecté.
pub fn read_config_cmd(info: &CaddyInfo) -> String {
    if info.mode == "docker" {
        match (&info.container, &info.config_dst) {
            (Some(cont), Some(dst)) => {
                format!("docker exec {} cat {} 2>/dev/null", quote(cont), quote(dst))
            }
            _ => match &info.config_src {
                Some(src) => format!(
                    "if [ \"$(id -u)\" = 0 ]; then cat {s} 2>/dev/null; else sudo -n cat {s} 2>/dev/null || cat {s} 2>/dev/null; fi",
                    s = quote(src)
                ),
                None => "true".to_string(),
            },
        }
    } else {
        "cat /etc/caddy/Caddyfile 2>/dev/null".to_string()
    }
}

/// Parse (grossièrement) un Caddyfile pour en extraire les liaisons (domaine -> port).
pub fn parse_caddyfile(raw: &str) -> Vec<CaddyRoute> {
    let mut routes = Vec::new();
    let mut depth = 0i32;
    let mut addr = String::new();
    let mut ssl = String::new();
    let mut port: Option<u16> = None;
    let mut pending_managed = false;
    let mut managed = false;

    for line in raw.lines() {
        let l = line.trim();
        if l.starts_with("# BEACON:") {
            if l.ends_with("START") {
                pending_managed = true;
            }
            continue;
        }
        if l.is_empty() || l.starts_with('#') {
            continue;
        }

        if depth == 0 {
            if let Some(idx) = l.find('{') {
                let head = l[..idx].trim();
                if !head.is_empty() && !head.starts_with('(') {
                    addr = head.to_string();
                    ssl = if addr.starts_with("http://") {
                        "none".into()
                    } else {
                        "public".into()
                    };
                    port = None;
                    managed = pending_managed;
                }
                pending_managed = false;
            }
            depth += l.matches('{').count() as i32 - l.matches('}').count() as i32;
            continue;
        }

        if l.contains("reverse_proxy") {
            if let Some(rest) = l.split("reverse_proxy").nth(1) {
                if let Some(tok) = rest.split_whitespace().next() {
                    let t = tok
                        .trim_start_matches("http://")
                        .trim_start_matches("https://");
                    if let Some(p) = t
                        .rsplit(':')
                        .next()
                        .and_then(|s| s.trim_end_matches('{').trim().parse::<u16>().ok())
                    {
                        port = Some(p);
                    }
                }
            }
        }
        if l.contains("tls internal") {
            ssl = "local".into();
        }

        let before = depth;
        depth += l.matches('{').count() as i32 - l.matches('}').count() as i32;
        if before > 0 && depth <= 0 {
            depth = 0;
            let domain = addr
                .split(',')
                .next()
                .unwrap_or(&addr)
                .trim()
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            if !domain.is_empty() && !domain.starts_with(':') && domain.contains('.') {
                routes.push(CaddyRoute {
                    domain,
                    target_port: port.unwrap_or(80),
                    ssl: if ssl.is_empty() {
                        "public".into()
                    } else {
                        ssl.clone()
                    },
                    managed,
                });
            }
            managed = false;
            addr.clear();
        }
    }
    routes
}

/// Domaines gérés par Beacon (repérés par les marqueurs) dans le texte du Caddyfile.
fn managed_domains(text: &str) -> std::collections::HashSet<String> {
    text.lines()
        .filter_map(|l| {
            l.trim()
                .strip_prefix("# BEACON:")
                .and_then(|s| s.strip_suffix(" START"))
                .map(|d| d.trim().to_string())
        })
        .collect()
}

/// Extrait (domaine, port) depuis la config JSON réellement chargée par Caddy (imports inclus).
fn extract_json(raw: &str) -> Option<Vec<(String, u16)>> {
    let v: serde_json::Value = serde_json::from_str(raw.trim()).ok()?;
    let servers = v.pointer("/apps/http/servers")?.as_object()?;
    let mut out = Vec::new();
    for srv in servers.values() {
        if let Some(routes) = srv.get("routes").and_then(|r| r.as_array()) {
            walk_json_routes(routes, &mut out);
        }
    }
    Some(out)
}

fn walk_json_routes(routes: &[serde_json::Value], out: &mut Vec<(String, u16)>) {
    for route in routes {
        let hosts: Vec<String> = route
            .get("match")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("host").and_then(|h| h.as_array()))
                    .flatten()
                    .filter_map(|h| h.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let port = route
            .get("handle")
            .and_then(find_dial)
            .and_then(|d| d.rsplit(':').next().and_then(|p| p.parse::<u16>().ok()))
            .unwrap_or(0);
        for h in &hosts {
            if h.contains('.') {
                out.push((h.clone(), port));
            }
        }
        if let Some(handle) = route.get("handle").and_then(|h| h.as_array()) {
            for handler in handle {
                if handler.get("handler").and_then(|x| x.as_str()) == Some("subroute") {
                    if let Some(sub) = handler.get("routes").and_then(|r| r.as_array()) {
                        walk_json_routes(sub, out);
                    }
                }
            }
        }
    }
}

fn find_dial(handle: &serde_json::Value) -> Option<String> {
    for h in handle.as_array()? {
        match h.get("handler").and_then(|x| x.as_str()) {
            Some("reverse_proxy") => {
                if let Some(dial) = h
                    .get("upstreams")
                    .and_then(|u| u.as_array())
                    .and_then(|a| a.first())
                    .and_then(|u| u.get("dial"))
                    .and_then(|d| d.as_str())
                {
                    return Some(dial.to_string());
                }
            }
            Some("subroute") => {
                if let Some(routes) = h.get("routes").and_then(|r| r.as_array()) {
                    for r in routes {
                        if let Some(d) = r.get("handle").and_then(find_dial) {
                            return Some(d);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Commande qui renvoie le Caddyfile (marqueurs) + la config JSON chargée (tous les domaines).
pub fn read_all_cmd(info: &CaddyInfo) -> String {
    let text = read_config_cmd(info);
    let json = if info.mode == "docker" {
        match &info.container {
            Some(cont) => {
                let dst = info
                    .config_dst
                    .clone()
                    .unwrap_or_else(|| "/etc/caddy/Caddyfile".into());
                format!(
                    "docker exec {c} sh -c 'wget -qO- http://localhost:2019/config/ 2>/dev/null || caddy adapt --config {d} 2>/dev/null'",
                    c = quote(cont),
                    d = dst
                )
            }
            None => "true".to_string(),
        }
    } else {
        "sh -c 'wget -qO- http://localhost:2019/config/ 2>/dev/null || caddy adapt --config /etc/caddy/Caddyfile 2>/dev/null'".to_string()
    };
    format!("echo '===TEXT==='\n{text}\necho '===JSON==='\n{json}")
}

/// Parse la sortie de `read_all_cmd` : domaines depuis le JSON (robuste), managed depuis le texte.
pub fn parse_routes(raw: &str) -> Vec<CaddyRoute> {
    let (text, json) = match raw.split_once("===JSON===") {
        Some((t, j)) => (t.replace("===TEXT===", ""), j.to_string()),
        None => (raw.to_string(), String::new()),
    };
    let managed = managed_domains(&text);

    if let Some(list) = extract_json(&json) {
        if !list.is_empty() {
            let mut seen = std::collections::HashSet::new();
            return list
                .into_iter()
                .filter(|(d, _)| seen.insert(d.clone()))
                .map(|(domain, target_port)| CaddyRoute {
                    managed: managed.contains(&domain),
                    domain,
                    target_port,
                    ssl: "public".into(),
                })
                .collect();
        }
    }

    // Repli : parsing texte du Caddyfile.
    let mut routes = parse_caddyfile(&text);
    for r in &mut routes {
        r.managed = managed.contains(&r.domain);
    }
    routes
}

/// Fichier Caddyfile à éditer (hôte) + commande de reload, selon le mode.
fn target_and_reload(info: &CaddyInfo) -> Result<(String, String), String> {
    if info.mode == "docker" {
        let src = info.config_src.clone().ok_or_else(|| {
            "Le Caddyfile n'est pas monté depuis l'hôte : Beacon ne peut pas modifier la config sans risque.".to_string()
        })?;
        let cont = info
            .container
            .clone()
            .ok_or_else(|| "Conteneur Caddy introuvable".to_string())?;
        let dst = info
            .config_dst
            .clone()
            .unwrap_or_else(|| "/etc/caddy/Caddyfile".into());
        let reload = format!(
            "docker exec {c} caddy reload --config {d} --adapter caddyfile 2>/dev/null || docker restart {c}",
            c = quote(&cont),
            d = quote(&dst)
        );
        Ok((src, reload))
    } else {
        let reload = "$SUDO caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile && ($SUDO systemctl reload caddy 2>/dev/null || $SUDO systemctl restart caddy)".to_string();
        Ok(("/etc/caddy/Caddyfile".into(), reload))
    }
}

/// Ajoute une liaison Beacon (bloc encadré de marqueurs) sans toucher au reste du fichier.
pub fn add_cmd(info: &CaddyInfo, route: &CaddyRoute) -> Result<String, String> {
    if !valid_domain(&route.domain) {
        return Err("Domaine invalide".into());
    }
    let (file, reload) = target_and_reload(info)?;
    let block = block_for(route);
    Ok(format!(
        "set -e\n\
         if [ \"$(id -u)\" = 0 ]; then SUDO=\"\"; else SUDO=\"sudo -n\"; fi\n\
         F={f}\n\
         $SUDO mkdir -p \"$(dirname \"$F\")\"\n\
         [ -f \"$F\" ] || : | $SUDO tee \"$F\" >/dev/null\n\
         if $SUDO grep -qF {domain} \"$F\"; then echo EXISTS; exit 0; fi\n\
         cat <<'BEACON_ADD_EOF' | $SUDO tee -a \"$F\" >/dev/null\n\
         # BEACON:{d} START\n\
         {block}# BEACON:{d} END\n\
         BEACON_ADD_EOF\n\
         {reload}\n\
         echo DONE",
        f = quote(&file),
        domain = quote(&route.domain),
        d = route.domain,
        block = block,
        reload = reload
    ))
}

/// Supprime uniquement le bloc Beacon d'un domaine (n'affecte pas les blocs de l'utilisateur).
pub fn remove_cmd(info: &CaddyInfo, domain: &str) -> Result<String, String> {
    if !valid_domain(domain) {
        return Err("Domaine invalide".into());
    }
    let (file, reload) = target_and_reload(info)?;
    let esc = domain.replace('.', "\\.");
    Ok(format!(
        "set -e\n\
         if [ \"$(id -u)\" = 0 ]; then SUDO=\"\"; else SUDO=\"sudo -n\"; fi\n\
         F={f}\n\
         $SUDO sed -i '/^# BEACON:{esc} START$/,/^# BEACON:{esc} END$/d' \"$F\"\n\
         {reload}\n\
         echo DONE",
        f = quote(&file),
        esc = esc,
        reload = reload
    ))
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

/// Installe Caddy comme conteneur Docker (géré par Beacon), avec Caddyfile monté depuis l'hôte.
pub const INSTALL_CMD: &str = "set -e\n\
     if ! command -v docker >/dev/null 2>&1; then echo NO_DOCKER; exit 1; fi\n\
     if [ \"$(id -u)\" = 0 ]; then SUDO=\"\"; else SUDO=\"sudo -n\"; fi\n\
     $SUDO mkdir -p /etc/caddy\n\
     [ -f /etc/caddy/Caddyfile ] || echo '# Géré par Beacon' | $SUDO tee /etc/caddy/Caddyfile >/dev/null\n\
     docker rm -f beacon-caddy 2>/dev/null || true\n\
     docker run -d --name beacon-caddy --restart unless-stopped \
       -p 80:80 -p 443:443 \
       -v /etc/caddy/Caddyfile:/etc/caddy/Caddyfile \
       -v beacon-caddy-data:/data -v beacon-caddy-config:/config \
       caddy:2\n\
     echo DONE";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteHealth {
    pub domain: String,
    pub dns_ok: bool,
    /// Le domaine répond via Caddy (test HTTPS local avec SNI).
    pub reachable: bool,
    pub http_code: String,
    pub resolved_ip: String,
    pub server_ip: String,
}

/// Diagnostic : IP publique + (DNS pointe ici ?) + le domaine répond-il via Caddy ?
/// On teste `https://<domaine>` en forçant la résolution vers 127.0.0.1 (le Caddy local),
/// ce qui reflète la vraie disponibilité quel que soit l'upstream (conteneur, service…).
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
            "echo \"DNS $(getent hosts '{d}' 2>/dev/null | awk '{{print $1}}' | head -1)\"\n",
            d = r.domain
        ));
        c.push_str(&format!(
            "echo \"CODE $(curl -sk -o /dev/null -w '%{{http_code}}' --max-time 6 --resolve '{d}:443:127.0.0.1' 'https://{d}' 2>/dev/null)\"\n",
            d = r.domain
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
                reachable: false,
                http_code: String::new(),
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
        } else if let Some(code) = l.strip_prefix("CODE ") {
            if let Some(r) = cur.as_mut() {
                let code = code.trim().to_string();
                r.reachable = code
                    .parse::<u32>()
                    .map(|n| (200..500).contains(&n))
                    .unwrap_or(false);
                r.http_code = code;
            }
        }
    }
    if let Some(r) = cur.take() {
        out.push(r);
    }
    out
}
