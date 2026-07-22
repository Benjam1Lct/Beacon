// Registre local des applications installées via l'App Store (par serveur).
// Chaque app déployée devient une tuile dans la section Applications.

export interface InstalledApp {
  id: string;
  catalogId: string; // id du catalogue (= nom de l'icône dans static/icons)
  label: string;
  name: string; // nom du conteneur / de la stack
  port: number | null;
  stack: boolean; // true = déployée via docker compose
}

const key = (pid: string) => `beacon.apps.${pid}`;

export function loadApps(pid: string): InstalledApp[] {
  try {
    return JSON.parse(localStorage.getItem(key(pid)) || "[]");
  } catch {
    return [];
  }
}

export function saveApps(pid: string, apps: InstalledApp[]) {
  try {
    localStorage.setItem(key(pid), JSON.stringify(apps));
  } catch {
    /* ignore */
  }
}

export function addApp(pid: string, app: InstalledApp) {
  const apps = loadApps(pid).filter((a) => a.name !== app.name);
  apps.push(app);
  saveApps(pid, apps);
}

export function removeApp(pid: string, id: string) {
  saveApps(
    pid,
    loadApps(pid).filter((a) => a.id !== id),
  );
}
