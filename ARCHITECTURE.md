# RustVPS — Architecture technique

> Dashboard desktop pour piloter un VPS avec une interface **style ZimaOS / CasaOS** :
> homelab moderne et épuré, orienté cartes d'apps et widgets.
> Backend Rust (Tauri) + frontend web. Connexion au VPS par **SSH pur, sans agent**.

---

## 1. Vision produit

Rendre l'administration d'un VPS accessible aux juniors via une interface **ZimaOS-like** :
un dashboard clair (grille de cartes d'apps + widgets de ressources), un **app store** pour
déployer des services en 1 clic, et une gestion **visuelle** des containers, du monitoring et
du réseau (reverse proxy). Zéro terminal, zéro config Nginx à la main.

### Direction UI (référence ZimaOS)

- **Dashboard d'accueil** : header avec widgets ressources (CPU / RAM / disque / réseau),
  puis une **grille de cartes** — une carte = une app/container installé (icône, statut,
  bouton ouvrir/stop). Fond doux, coins arrondis, ombres légères, mode clair/sombre.
- **App Store** : catalogue en cartes par catégorie (Databases, Media, Tools…), bouton
  "Installer" qui déploie un container préconfiguré.
- **Réseau** : éditeur visuel de reverse proxy + SSL (voir §11).
- **Files / Terminal / Settings** : entrées d'un dock ou menu latéral, comme ZimaOS.
- Alertes discrètes mais claires : badge rouge + message en langage humain quand un service
  plante ou qu'une ressource sature, avec bouton d'action 1-clic.

### Actions 1-clic (par carte)

| État réel VPS                     | Signal dans l'UI                          | Action 1-clic proposée        |
|-----------------------------------|-------------------------------------------|-------------------------------|
| Container Docker `running`        | Carte verte, statut "Up"                   | Ouvrir / Voir logs            |
| Container `exited` / crash        | Carte grise + badge rouge                  | Redémarrer                    |
| CPU / RAM saturés                 | Widget rouge + message clair               | Voir top process              |
| Disque plein                      | Widget disque rouge                        | Nettoyer logs & images        |
| Logs volumineux                   | Alerte "logs volumineux"                    | `docker logs` truncate / prune|
| App store                         | Catalogue de cartes installables           | Déployer (MongoDB, Redis…)    |

---

## 2. Stack technique

### Backend (Rust)
- **Tauri v2** — shell desktop, IPC vers le frontend, gestion des fenêtres.
- **russh** (+ `russh-keys`) — client SSH pur Rust (async), exécution de commandes distantes.
  - Alternative si besoin de robustesse : wrapper le binaire `ssh` système. On part sur `russh`.
- **tokio** — runtime async (SSH, monitoring périodique).
- **serde / serde_json** — sérialisation des structures échangées avec le frontend.
- **keyring** — stockage sécurisé des secrets (clé/passphrase SSH) dans le trousseau de l'OS
  (Windows Credential Manager / macOS Keychain / libsecret).
- **thiserror** — types d'erreurs propres.
- **tracing** — logs applicatifs.

### Frontend (web)
- **Vite** + **TypeScript**.
- Framework UI : **Svelte** (léger, réactif, parfait pour un dashboard) — ou React si préféré.
- Rendu "ville" : **Canvas 2D** (via PixiJS) pour l'iso/animations, ou simple CSS grid au début.
- Charts jauges : composants maison (SVG) ou une petite lib légère.

### Communication
- Frontend ↔ Backend : **commandes Tauri** (`#[tauri::command]`) + **events** (push temps réel).
- Backend ↔ VPS : **SSH**, commandes shell dont on parse la sortie (JSON quand possible :
  `docker ... --format '{{json .}}'`).

---

## 3. Structure du projet

```
RustVPS/
├── ARCHITECTURE.md              # ce fichier
├── package.json                 # frontend (Vite/Svelte)
├── index.html
├── vite.config.ts
├── src/                         # FRONTEND
│   ├── main.ts
│   ├── App.svelte
│   ├── lib/
│   │   ├── api.ts               # wrappers invoke() vers les commandes Tauri
│   │   ├── types.ts             # types partagés (miroir des structs Rust)
│   │   └── stores.ts            # état réactif (serveurs, métriques, containers)
│   ├── components/
│   │   ├── Dashboard.svelte     # accueil: widgets + grille de cartes
│   │   ├── AppCard.svelte       # une carte = un container/app
│   │   ├── Gauges.svelte        # widgets RAM / CPU / disque / réseau
│   │   ├── AppStore.svelte      # catalogue déployable
│   │   ├── ProxyEditor.svelte   # éditeur visuel reverse proxy (câbles)
│   │   └── ServerConnect.svelte # écran de connexion SSH (IP + import clé)
│   └── styles/
└── src-tauri/                   # BACKEND RUST
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    └── src/
        ├── main.rs              # setup Tauri, enregistrement des commandes
        ├── state.rs             # état partagé (connexions SSH actives)
        ├── ssh/
        │   ├── mod.rs
        │   ├── client.rs        # connexion russh, exec_command()
        │   └── secrets.rs       # keyring: save/load creds
        ├── monitor/
        │   ├── mod.rs
        │   ├── metrics.rs       # parse free/df/uptime/loadavg
        │   └── poller.rs        # boucle tokio -> emit events
        ├── docker/
        │   ├── mod.rs
        │   ├── containers.rs    # ps / start / stop / restart / logs
        │   └── deploy.rs        # docker run à partir d'un template
        ├── appstore/
        │   ├── mod.rs
        │   └── catalog.rs       # définitions MongoDB, Redis, Postgres…
        ├── proxy/
        │   ├── mod.rs
        │   ├── model.rs         # Route (domaine -> container:port)
        │   ├── caddy.rs         # génération/reload Caddyfile (SSL auto)
        │   └── health.rs        # test lien: DNS, port ouvert, cert valide
        ├── hardening/
        │   ├── mod.rs
        │   ├── wizard.rs        # orchestration du durcissement (§12), étape par étape
        │   ├── keygen.rs        # génération paire de clés Ed25519 (russh-keys)
        │   ├── user.rs          # créer user dev + sudo, pousser la clé publique
        │   └── sshd.rs          # drop-in sshd_config, reload, vérif anti-lockout
        └── commands.rs          # #[tauri::command] exposés au frontend
```

---

## 4. Modèle de données (contrats front/back)

```rust
// Serveur enregistré
struct ServerProfile {
    id: String,
    label: String,
    host: String,       // IP saisie par l'utilisateur
    port: u16,          // 22
    username: String,
    auth: AuthMethod,
}

enum AuthMethod {
    // Clé importée: la clé privée est copiée dans le keyring OS (pas juste référencée).
    ImportedKey { key_id: String, has_passphrase: bool },
    // Clé générée par l'app lors du durcissement (§12), stockée dans le keyring.
    ManagedKey { key_id: String },
    // Réutiliser une clé déjà chargée dans l'agent SSH du système.
    Agent,
    // Mot de passe: UNIQUEMENT pour le tout premier contact (bootstrap §12).
    // Jamais persisté — gardé en mémoire le temps de la session de durcissement.
    BootstrapPassword,
}

// Métriques (poll périodique)
struct Metrics {
    cpu_percent: f32,
    load_avg: [f32; 3],
    mem_used_mb: u64,
    mem_total_mb: u64,
    disk_used_gb: u64,
    disk_total_gb: u64,
    uptime_secs: u64,
    timestamp: u64,
}

// Container Docker
struct Container {
    id: String,
    name: String,
    image: String,
    state: String,      // running | exited | restarting…
    status: String,     // "Up 3 hours"
    cpu_percent: f32,
    mem_mb: u64,
    ports: Vec<String>,
}

// Entrée App Store
struct AppTemplate {
    id: String,             // "mongodb"
    name: String,           // "MongoDB"
    category: String,       // database | cache | tool…
    image: String,          // "mongo:7"
    default_ports: Vec<PortMap>,
    env: Vec<EnvVar>,       // dont secrets à générer
    volumes: Vec<String>,
    description: String,
}

// Route reverse proxy (§11)
struct ProxyRoute {
    id: String,
    domain: String,          // "monbot.com"
    target_container: String, // id/nom du container
    target_port: u16,        // 8080
    ssl: bool,               // Let's Encrypt auto
    status: LinkStatus,      // Ok | DnsError | PortClosed | CertError | Pending
    last_checked: u64,
}

enum LinkStatus {            // -> couleur du câble dans l'UI
    Ok,                      // vert
    Pending,                 // jaune (SSL en cours d'émission)
    DnsError,                // rouge: le domaine ne pointe pas vers le VPS
    PortClosed,              // rouge: le container n'écoute pas sur ce port
    CertError,               // rouge: échec Let's Encrypt (rate limit, port 80…)
}
```

---

## 5. Commandes SSH utilisées (parsing)

| But               | Commande distante                                              |
|-------------------|---------------------------------------------------------------|
| Mémoire           | `free -m`                                                     |
| Disque            | `df -B1 /`                                                    |
| CPU / load        | `cat /proc/loadavg` + `top -bn1` (ou `mpstat`)               |
| Uptime            | `cat /proc/uptime`                                           |
| Docker list       | `docker ps -a --format '{{json .}}'`                          |
| Docker stats      | `docker stats --no-stream --format '{{json .}}'`             |
| Start/Stop        | `docker start|stop|restart <id>`                              |
| Logs              | `docker logs --tail 200 <id>`                                 |
| Déployer          | `docker run -d --name … -p … -e … -v … <image>`             |
| Nettoyage         | `docker system prune -f` / `truncate -s 0 <logfile>`         |

> Toutes les commandes destructives (`prune`, `stop`, `rm`, nettoyage logs) passent par
> une **confirmation explicite dans l'UI** avant exécution.

---

## 6. Sécurité — 100% local

Principe : **rien ne sort de la machine de l'utilisateur**. L'app desktop parle directement
au VPS en SSH. Pas de backend cloud, pas de relais, pas de télémétrie. La seule connexion
sortante est desktop → VPS.

### Flux de connexion (M1)
1. L'utilisateur saisit l'**IP** (+ port/username, 22/root par défaut).
2. Deux cas d'entrée :
   - **Il a déjà une clé** → il l'importe (`id_ed25519` / `id_rsa`, sélecteur de fichier).
   - **Il n'a que root + mot de passe** → connexion bootstrap par mot de passe, puis l'app
     propose immédiatement le **durcissement (§12)** qui génère une clé et sécurise le VPS.
3. La clé (importée ou générée) est **copiée dans le keyring de l'OS** (Windows Credential
   Manager / macOS Keychain / libsecret) — jamais en clair sur disque, jamais loggée.
4. Au 1er contact, la **host key du VPS est affichée et épinglée (TOFU)** ; si elle change
   plus tard → alerte MITM et blocage.
5. Test de connexion (`uname -a`) → profil enregistré.

### Garde-fous
- **Aucun agent** installé sur le VPS : surface d'attaque minimale, rien à maintenir côté serveur.
- Secrets (clé privée, passphrase) → **keyring OS** uniquement. Passphrase demandée à l'usage,
  pas stockée en clair.
- **Vérification/pinning de la host key** pour éviter le man-in-the-middle.
- Auth par **clé SSH** uniquement (pas de mot de passe persisté).
- Whitelist des commandes destructives (`prune`, `stop`, `rm`, clean logs) + **confirmation UI**.
- Timeouts et gestion d'erreurs sur toutes les commandes distantes.
- **Zéro donnée envoyée à un tiers** : tout transite en local, chiffré par SSH de bout en bout.

---

## 7. Flux temps réel

```
[poller.rs boucle tokio 5s]
   -> exec SSH (free/df/loadavg + docker stats)
   -> parse -> Metrics + Vec<Container>
   -> app_handle.emit("metrics:update", payload)
[Frontend] onEvent("metrics:update") -> maj stores -> re-render cartes + widgets
```

---

## 8. Roadmap (jalons)

- **M0 — Scaffold** : projet Tauri v2 + Svelte qui build et ouvre une fenêtre.
- **M1 — Connexion SSH** : écran de connexion, russh, "hello" (`uname -a`), creds au keyring.
- **M1.5 — Durcissement first-run** : wizard user dev + clé + désactivation password/root (§12).
- **M2 — Monitoring** : poller + widgets RAM/CPU/disque live (dashboard ZimaOS-like).
- **M3 — Docker** : grille de cartes containers + start/stop/restart/logs.
- **M4 — App Store** : catalogue (MongoDB, Redis, Postgres) + déploiement 1-clic.
- **M5 — Reverse Proxy visuel** : éditeur câbles domaine→container + SSL auto (§11).
- **M6 — Polish** : multi-serveurs, dark mode, actions 1-clic (prune, clean logs), thèmes.

---

## 11. Reverse Proxy & SSL visuel

> L'alternative fun à Nginx Proxy Manager : relier un domaine à un container en tirant un
> câble, et le SSL se fait tout seul en tâche de fond.

### Concept UI

- Deux colonnes : à gauche les **domaines**, à droite les **containers** (avec leurs ports).
- L'utilisateur **tire un câble** de `monbot.com` vers `mon-container:8080`.
- Le câble prend une **couleur selon l'état du lien** (voir `LinkStatus`) :
  - 🟢 vert = tout marche (DNS ok, port ouvert, cert valide)
  - 🟡 jaune = SSL en cours d'émission
  - 🔴 rouge clignotant = problème, avec **message en langage humain** au survol :
    - *"Ton domaine ne pointe pas vers ce VPS. Ajoute un enregistrement A vers `1.2.3.4`."*
    - *"Le container n'écoute pas sur le port 8080."*
    - *"Let's Encrypt a échoué : le port 80 doit être ouvert pour valider le certificat."*

### Moteur : Caddy (recommandé plutôt que Nginx)

**Pourquoi Caddy** : SSL automatique (ACME/Let's Encrypt intégré, renouvellement auto), config
minimaliste, reload à chaud. Le drame des certificats qui expirent disparaît par design.

Flux :
```
UI (tire un câble) -> ProxyRoute
  -> proxy/caddy.rs génère un bloc Caddyfile:
         monbot.com {
             reverse_proxy 127.0.0.1:8080
         }
  -> upload du Caddyfile via SSH + `caddy reload` (ou API admin de Caddy)
  -> Caddy obtient/renouvelle le cert automatiquement
  -> proxy/health.rs vérifie le lien et met à jour LinkStatus
```

> Alternative si l'utilisateur tient à Nginx : générer des `server {}` + gérer `certbot`.
> Plus de code et de cas d'erreur — **on part sur Caddy** pour le MVP.

### Diagnostic automatique (`proxy/health.rs`)

Pour colorer le câble et expliquer en clair, on teste, dans l'ordre :
1. **DNS** : le domaine résout-il vers l'IP du VPS ? (`dig +short monbot.com`)
2. **Port container** : le service écoute-t-il ? (`ss -ltn` / `docker port`)
3. **HTTP** : le reverse proxy répond-il ? (`curl -I http://127.0.0.1:PORT`)
4. **Cert** : certificat présent et non expiré ? (état Caddy / `openssl s_client`)

Chaque échec → un `LinkStatus` + un message pré-rédigé, pas un log brut.

### Prérequis / garde-fous

- Caddy doit tourner sur le VPS (le proposer à l'installation via l'App Store — "1-clic").
- Ports **80 et 443** ouverts (nécessaires pour ACME) — l'app le vérifie et prévient sinon.
- Toute écriture de config passe par une **prévisualisation + confirmation** dans l'UI.

---

## 9. Points à trancher plus tard

- Framework front définitif : **Svelte** (proposé) vs React.
- Rendu cartes/proxy : CSS + SVG (rapide) — canvas seulement si besoin.
- Multi-serveurs dès M1 ou après.
- Nom du user dev créé au durcissement (fixe `dev` vs choisi par l'utilisateur).

---

## 12. Durcissement & onboarding first-run

> À la première connexion (souvent root + mot de passe), l'app propose de **sécuriser le VPS
> automatiquement** : créer un utilisateur dédié, générer une clé SSH, désactiver le login par
> mot de passe et le login root. C'est un wizard clair, réversible, avec garde-fou anti-lockout.

### Règle d'or (anti-verrouillage)

> **On ne désactive JAMAIS le mot de passe ni le root login avant d'avoir prouvé que la nouvelle
> connexion par clé fonctionne.** Chaque étape destructive est précédée d'un test de la nouvelle
> voie d'accès. En cas d'échec, rollback automatique.

### Étapes du wizard (`hardening/wizard.rs`)

1. **Connexion bootstrap** en root (clé si dispo, sinon mot de passe — non persisté).
2. **Générer une paire de clés Ed25519** en local (`keygen.rs`) → clé privée direct au keyring.
3. **Créer l'utilisateur dev** (`user.rs`) :
   - `adduser --disabled-password dev`
   - ajouter aux groupes `sudo` (ou `wheel`) et `docker`
   - sudo sans mot de passe optionnel : drop-in `/etc/sudoers.d/` (validé avec `visudo -c`)
4. **Pousser la clé publique** : créer `~dev/.ssh/authorized_keys` (perms `700` dossier / `600`
   fichier, `chown dev:dev`).
5. **✅ Test crucial** : ouvrir une **nouvelle session SSH `dev@ip` avec la clé générée** et
   lancer `sudo -n true`. Si KO → on s'arrête, rien n'est désactivé, on explique pourquoi.
6. **Durcir sshd** (`sshd.rs`) via un drop-in `/etc/ssh/sshd_config.d/99-rustvps.conf` :
   ```
   PermitRootLogin no
   PasswordAuthentication no
   PubkeyAuthentication yes
   ```
   - Valider la syntaxe : `sshd -t`
   - Recharger : `systemctl reload sshd` (fallback `service ssh reload`)
7. **Re-test post-reload** : nouvelle session `dev` par clé toujours OK ? Sinon **rollback**
   (supprimer le drop-in + reload) et alerter.
8. **Bascule du profil** : l'app passe en `ManagedKey` + user `dev`, oublie le mot de passe.

### Garde-fous

- Wizard **opt-in**, chaque étape affichée en clair avec ce qu'elle va faire.
- **Aperçu** des fichiers modifiés (sshd drop-in, sudoers) avant application.
- Le drop-in `99-rustvps.conf` rend le durcissement **traçable et réversible** (un seul fichier
  à retirer), sans toucher au `sshd_config` d'origine.
- Rollback automatique à la moindre perte d'accès par clé.
- Option de **garder le mot de passe root actif** en repli tant que l'utilisateur n'a pas
  confirmé plusieurs reconnexions réussies (désactivation "à froid" plus tard).
- Idempotent : relancer le wizard ne casse rien (vérifie l'existant avant d'agir).

### Ce que ça donne côté sécurité VPS

Après le wizard : plus de root SSH, plus de brute-force possible sur mot de passe, accès
uniquement par la clé détenue localement dans le keyring. C'est le durcissement standard
d'un VPS, mais fait **en 1 clic et sans terminal**.
