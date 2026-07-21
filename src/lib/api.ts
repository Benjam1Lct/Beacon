// Wrappers typés autour des commandes Tauri (invoke) et du plugin dialog.

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ExecOutcome,
  HardenInput,
  HardeningReport,
  Metrics,
  ProfileMeta,
  SaveProfileInput,
  SshProfile,
} from "./types";

/** Teste une connexion ad-hoc ; renvoie la sortie de `uname -a` + l'empreinte de la clé d'hôte. */
export function sshTestConnection(profile: SshProfile): Promise<ExecOutcome> {
  return invoke<ExecOutcome>("ssh_test_connection", { profile });
}

/** Liste les profils enregistrés (métadonnées locales, sans secret). */
export function listProfiles(): Promise<ProfileMeta[]> {
  return invoke<ProfileMeta[]>("list_profiles");
}

/** Enregistre un profil (clé rangée dans le keyring, métadonnées en JSON local). */
export function saveProfile(input: SaveProfileInput): Promise<ProfileMeta> {
  return invoke<ProfileMeta>("save_profile", { input });
}

/** Supprime un profil et son secret. */
export function deleteProfile(id: string): Promise<void> {
  return invoke<void>("delete_profile", { id });
}

/** Se connecte à un profil enregistré (TOFU appliqué). Mot de passe requis pour ce type de profil. */
export function connectProfile(id: string, password?: string): Promise<ExecOutcome> {
  return invoke<ExecOutcome>("connect_profile", { id, password: password ?? null });
}

/** Récupère les métriques système d'un serveur (CPU/RAM/disque/réseau). */
export function fetchMetrics(id: string, password?: string): Promise<Metrics> {
  return invoke<Metrics>("fetch_metrics", { id, password: password ?? null });
}

/** Durcissement first-run (root uniquement) : crée un user dédié, clé, désactive root/password. */
export function hardenBootstrap(input: HardenInput): Promise<HardeningReport> {
  return invoke<HardeningReport>("harden_bootstrap", { input });
}

/** Ouvre un sélecteur de fichier pour choisir une clé SSH privée. Renvoie le chemin ou null. */
export async function pickKeyFile(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    title: "Choisir une clé SSH privée",
  });
  return typeof selected === "string" ? selected : null;
}
