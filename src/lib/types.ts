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

export interface HardenInput {
  host: string;
  port: number;
  rootUsername: string;
  auth: AuthInput;
  devUsername: string;
  label: string;
}

export interface HardeningStep {
  key: string;
  label: string;
  status: "ok" | "skipped" | "failed";
  detail?: string | null;
}

export interface HardeningReport {
  success: boolean;
  steps: HardeningStep[];
  profile: ProfileMeta | null;
  message: string;
}

export interface Container {
  id: string;
  name: string;
  image: string;
  state: string;
  status: string;
  ports: string;
  cpuPercent: number | null;
  memUsage: string | null;
}

export interface DockerStatus {
  installed: boolean;
  containers: Container[];
}

export interface DirEntry {
  name: string;
  isDir: boolean;
}
export interface DirListing {
  path: string;
  entries: DirEntry[];
}

export interface FilePreview {
  kind: "text" | "image" | "pdf" | "binary";
  name: string;
  mime: string;
  content: string;
  size: number;
  truncated: boolean;
}

export interface Process {
  pid: number;
  user: string;
  cpu: number;
  mem: number;
  command: string;
}

export interface CaddyInfo {
  installed: boolean;
  mode: "system" | "docker" | "none";
  container: string | null;
  configSrc: string | null;
  configDst: string | null;
}

export type SslMode = "public" | "local" | "none";
export interface CaddyRoute {
  domain: string;
  targetPort: number;
  ssl: SslMode;
  managed?: boolean;
}
export interface RouteHealth {
  domain: string;
  dnsOk: boolean;
  portOk: boolean;
  resolvedIp: string;
  serverIp: string;
}

export interface Metrics {
  hostname: string;
  uptimeSecs: number;
  cpuCores: number;
  cpuPercent: number;
  loadAvg: [number, number, number];
  memTotalKb: number;
  memAvailableKb: number;
  diskTotalBytes: number;
  diskUsedBytes: number;
  netRxBytes: number;
  netTxBytes: number;
}
