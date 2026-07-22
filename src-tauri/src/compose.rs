//! Déploiement d'applications multi-conteneurs via Docker Compose (§M4+).
//!
//! Beacon écrit un docker-compose.yml (issu du catalogue, valeurs substituées) dans un dossier
//! dédié sur le serveur, puis lance `docker compose up -d`.

/// Valide un nom de stack (dossier + projet compose).
pub fn valid_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 40
        && n.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        && n.chars()
            .next()
            .map(|c| c.is_ascii_lowercase())
            .unwrap_or(false)
}

/// Commande : écrit le compose et lance `docker compose up -d` (v2 ou v1).
pub fn up_cmd(name: &str, yaml: &str) -> Option<String> {
    if !valid_name(name) {
        return None;
    }
    Some(format!(
        "set -e\n\
         D=\"$HOME/beacon-stacks/{name}\"\n\
         mkdir -p \"$D\"\n\
         cat <<'BEACON_COMPOSE_EOF' > \"$D/docker-compose.yml\"\n\
         {yaml}\n\
         BEACON_COMPOSE_EOF\n\
         cd \"$D\"\n\
         if docker compose version >/dev/null 2>&1; then docker compose up -d; else docker-compose up -d; fi\n\
         echo DONE"
    ))
}

/// Commande : arrête et retire la stack (`docker compose down`).
pub fn down_cmd(name: &str) -> Option<String> {
    if !valid_name(name) {
        return None;
    }
    Some(format!(
        "D=\"$HOME/beacon-stacks/{name}\"\n\
         if [ -f \"$D/docker-compose.yml\" ]; then\n\
           cd \"$D\"\n\
           if docker compose version >/dev/null 2>&1; then docker compose down; else docker-compose down; fi\n\
         fi\n\
         echo DONE"
    ))
}
