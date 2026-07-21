// Wrappers typés autour des commandes Tauri (invoke).

import { invoke } from "@tauri-apps/api/core";
import type { SshProfile, SshResult } from "./types";

/** Teste une connexion SSH ; renvoie la sortie de `uname -a` ou lève l'erreur backend. */
export function sshTestConnection(profile: SshProfile): Promise<SshResult> {
  return invoke<SshResult>("ssh_test_connection", { profile });
}
