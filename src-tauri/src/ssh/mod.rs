//! Connexion SSH au serveur distant (VPS ou machine locale).
//!
//! M1 : établir une connexion et exécuter une commande (`uname -a` pour le test).
//! Client pur Rust (russh) — aucun binaire système requis, compile identique sur les 3 OS.

mod client;

pub use client::{exec, SshError, SshProfile, SshResult};
