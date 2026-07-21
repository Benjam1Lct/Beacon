// Types partagés avec le backend Rust (miroir des structs de src-tauri/src/ssh).

export type AuthInput =
  | { kind: "password"; password: string }
  | { kind: "key"; path: string; passphrase?: string | null };

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
