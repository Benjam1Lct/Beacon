# Beacon

> Pilote ton serveur Linux — **VPS cloud ou machine locale** — sans jamais toucher au terminal.

Beacon est une application **desktop** (Windows / Linux / macOS) qui se connecte à ton serveur
en SSH et t'en donne une interface claire, style [ZimaOS](https://www.zimaspace.com/) :
monitoring, gestion Docker, app store 1-clic, reverse proxy visuel et durcissement SSH automatique.

## Pourquoi tu peux lui faire confiance

Donner un accès SSH à une app, ça ne se fait pas à la légère. Beacon est construit pour être
vérifiable, pas pour te demander de la foi :

- **100 % local.** Aucune connexion cloud, aucun compte, aucune télémétrie. La seule connexion
  sortante est ton poste → ton serveur, chiffrée par SSH de bout en bout.
- **Code public et auditable.** Tu peux lire exactement ce que Beacon envoie sur ton serveur.
- **Chaque commande est visible.** Beacon affiche la commande SSH exacte avant les actions
  sensibles, et journalise ce qu'il exécute.
- **Moindre privilège.** Beacon peut créer un utilisateur dédié et te faire quitter le root.
- **Tout est réversible.** Ce que Beacon configure sur le serveur tient dans des fichiers
  identifiables, retirables à la main.

> **Licence :** [Business Source License 1.1](LICENSE). Le code est public et auditable, l'usage
> pour tes propres serveurs (même en production) est autorisé, mais en faire un produit
> commercial concurrent ne l'est pas. La licence bascule automatiquement en Apache 2.0 le
> 2030-07-21.

## Fonctionnalités (voir [ARCHITECTURE.md](ARCHITECTURE.md))

- 🔌 **Connexion SSH** par clé (import ou clé générée par l'app), secrets dans le keyring de l'OS.
- 🛡️ **Durcissement first-run** : crée un user dédié, désactive le login root et par mot de passe.
- 📊 **Monitoring** temps réel (CPU / RAM / disque / réseau).
- 🐳 **Docker** : cartes de containers, start / stop / restart / logs.
- 🏪 **App Store** : déploiement 1-clic (MongoDB, Redis, Postgres…).
- 🔗 **Reverse Proxy visuel** : relie un domaine à un container en tirant un câble, SSL auto (Caddy).

## Stack

- **Backend :** Rust + [Tauri 2](https://tauri.app) (SSH via `russh`, secrets via `keyring`).
- **Frontend :** SvelteKit + TypeScript (mode SPA statique).

## Développement

Prérequis : [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, et les
[dépendances système Tauri](https://tauri.app/start/prerequisites/) de ta plateforme.

```bash
npm install          # dépendances frontend
npm run tauri dev    # lance l'app en mode dev (hot reload)
npm run tauri build  # build de production local
```

### Note sur les plateformes

Le développement se fait principalement sous Windows. La compilation **Linux et macOS est
garantie par l'intégration continue** : le workflow [CI](.github/workflows/ci.yml) build et lint
le projet sur les 3 systèmes à chaque push. Les binaires officiels sont produits par le workflow
[Release](.github/workflows/release.yml) à chaque tag `v*`.

## Releases

Les binaires pour Windows, Linux et macOS (Intel + Apple Silicon) sont disponibles dans
l'onglet [Releases](../../releases). Ils sont générés automatiquement par GitHub Actions.

> Les builds ne sont pas encore signés : au premier lancement, Windows (SmartScreen) et macOS
> (Gatekeeper) peuvent afficher un avertissement.
