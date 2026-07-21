//! Connexion SSH au serveur distant (VPS ou machine locale).
//!
//! Client pur Rust (russh) — aucun binaire système requis, compile identique sur les 3 OS.
//! Gère le pinning de la clé d'hôte (TOFU) contre les attaques MITM.

mod client;

pub use client::{exec, AuthInput, ExecOutcome, SshProfile};
