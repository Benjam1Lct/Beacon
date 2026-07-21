// Types partagés avec le backend Rust (miroir des structs de src-tauri/src).

export type AuthInput =
  | { kind: "password"; password: string }
  | { kind: "key"; path: string; passphrase?: string | null }
  | { kind: "keyContent"; pem: string; passphrase?: string | null };

export interface SshProfile {
  host: string;
  port: number;
  username: string;
  auth: AuthInput;
}

export interface SshResult {
  stdout: string;
  stderr: string;
  exit_code: number;
}

export interface ExecOutcome {
  result: SshResult;
  host_key_fp: string;
}

export type AuthKind = "key" | "password";

/** Métadonnées d'un serveur enregistré (aucun secret). */
export interface ProfileMeta {
  id: string;
  label: string;
  host: string;
  port: number;
  username: string;
  authKind: AuthKind;
  hostKeyFp: string | null;
}

export type SaveAuth =
  | { kind: "key"; path: string; passphrase?: string | null }
  | { kind: "password" };

export interface SaveProfileInput {
  label: string;
  host: string;
  port: number;
  username: string;
  auth: SaveAuth;
  hostKeyFp?: string | null;
}
