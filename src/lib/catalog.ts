// Catalogue d'applications de l'App Store (intégré à Beacon, 100 % local).
// À l'installation, Docker pull l'image depuis Docker Hub et lance le conteneur.

export interface PortMap {
  host: number;
  container: number;
}
export interface EnvVar {
  key: string;
  value: string;
}
export interface DeployConfig {
  name: string;
  image: string;
  ports: PortMap[];
  env: EnvVar[];
  volumes: string[];
}

export interface CatEnv {
  key: string;
  value?: string;
  generate?: boolean; // mot de passe généré
  label?: string;
}
export interface CatPort {
  container: number;
  defaultHost: number;
}
export interface AppTemplate {
  id: string;
  name: string;
  mono: string; // monogramme de l'icône
  color: string;
  category: string;
  description: string;
  image: string;
  ports: CatPort[];
  env: CatEnv[];
  volumes: string[]; // {name} remplacé par le nom du conteneur
  /** Si présent : app multi-conteneurs (docker compose). Placeholders {{NAME}} {{PORT}} {{KEY}}. */
  compose?: string;
}

export const CATALOG: AppTemplate[] = [
  {
    id: "postgres",
    name: "PostgreSQL",
    mono: "PG",
    color: "#336791",
    category: "Base de données",
    description: "Base de données relationnelle robuste et populaire.",
    image: "postgres:16",
    ports: [{ container: 5432, defaultHost: 5432 }],
    env: [
      { key: "POSTGRES_USER", value: "admin" },
      { key: "POSTGRES_PASSWORD", generate: true, label: "Mot de passe" },
    ],
    volumes: ["{name}-data:/var/lib/postgresql/data"],
  },
  {
    id: "mysql",
    name: "MySQL",
    mono: "My",
    color: "#e48e00",
    category: "Base de données",
    description: "Base de données relationnelle très répandue.",
    image: "mysql:8",
    ports: [{ container: 3306, defaultHost: 3306 }],
    env: [{ key: "MYSQL_ROOT_PASSWORD", generate: true, label: "Mot de passe root" }],
    volumes: ["{name}-data:/var/lib/mysql"],
  },
  {
    id: "mongodb",
    name: "MongoDB",
    mono: "Mo",
    color: "#13aa52",
    category: "Base de données",
    description: "Base de données NoSQL orientée documents.",
    image: "mongo:7",
    ports: [{ container: 27017, defaultHost: 27017 }],
    env: [
      { key: "MONGO_INITDB_ROOT_USERNAME", value: "admin" },
      { key: "MONGO_INITDB_ROOT_PASSWORD", generate: true, label: "Mot de passe" },
    ],
    volumes: ["{name}-data:/data/db"],
  },
  {
    id: "redis",
    name: "Redis",
    mono: "Re",
    color: "#d82c20",
    category: "Cache",
    description: "Cache clé-valeur en mémoire, ultra-rapide.",
    image: "redis:7",
    ports: [{ container: 6379, defaultHost: 6379 }],
    env: [],
    volumes: ["{name}-data:/data"],
  },
  {
    id: "nginx",
    name: "Nginx",
    mono: "Nx",
    color: "#009639",
    category: "Web",
    description: "Serveur web et reverse proxy performant.",
    image: "nginx:latest",
    ports: [{ container: 80, defaultHost: 8080 }],
    env: [],
    volumes: [],
  },
  {
    id: "portainer",
    name: "Portainer",
    mono: "Pt",
    color: "#13bef9",
    category: "Outils",
    description: "Interface web de gestion Docker.",
    image: "portainer/portainer-ce:latest",
    ports: [{ container: 9000, defaultHost: 9000 }],
    env: [],
    volumes: ["/var/run/docker.sock:/var/run/docker.sock", "{name}-data:/data"],
  },
  {
    id: "uptime-kuma",
    name: "Uptime Kuma",
    mono: "Uk",
    color: "#5cdd8b",
    category: "Monitoring",
    description: "Surveillance de disponibilité (uptime) auto-hébergée.",
    image: "louislam/uptime-kuma:1",
    ports: [{ container: 3001, defaultHost: 3001 }],
    env: [],
    volumes: ["{name}-data:/app/data"],
  },
  {
    id: "n8n",
    name: "n8n",
    mono: "n8",
    color: "#ea4b71",
    category: "Automatisation",
    description: "Automatisation de workflows (type Zapier) auto-hébergée.",
    image: "n8nio/n8n:latest",
    ports: [{ container: 5678, defaultHost: 5678 }],
    env: [],
    volumes: ["{name}-data:/home/node/.n8n"],
  },
  {
    id: "adminer",
    name: "Adminer",
    mono: "Ad",
    color: "#34567c",
    category: "Outils",
    description: "Gestion de bases de données en une page web.",
    image: "adminer:latest",
    ports: [{ container: 8080, defaultHost: 8082 }],
    env: [],
    volumes: [],
  },
  {
    id: "umami",
    name: "Umami",
    mono: "Um",
    color: "#000000",
    category: "Analytics",
    description: "Analytics web (visiteurs, pays, référents) respectueux de la vie privée.",
    image: "ghcr.io/umami-software/umami:postgresql-latest",
    ports: [{ container: 3000, defaultHost: 3000 }],
    env: [
      { key: "PASSWORD", generate: true, label: "Mot de passe base de données" },
      { key: "SECRET", generate: true, label: "Clé secrète" },
    ],
    volumes: [],
    compose: `services:
  {{NAME}}:
    image: ghcr.io/umami-software/umami:postgresql-latest
    ports:
      - "{{PORT}}:3000"
    environment:
      DATABASE_URL: postgresql://umami:{{PASSWORD}}@db:5432/umami
      DATABASE_TYPE: postgresql
      APP_SECRET: "{{SECRET}}"
    depends_on:
      db:
        condition: service_healthy
    restart: unless-stopped
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: umami
      POSTGRES_USER: umami
      POSTGRES_PASSWORD: {{PASSWORD}}
    volumes:
      - {{NAME}}-db:/var/lib/postgresql/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U umami"]
      interval: 5s
      timeout: 5s
      retries: 10
volumes:
  {{NAME}}-db:
`,
  },
  {
    id: "wordpress",
    name: "WordPress",
    mono: "Wp",
    color: "#21759b",
    category: "Web",
    description: "Le CMS le plus utilisé, avec sa base de données.",
    image: "wordpress:latest",
    ports: [{ container: 80, defaultHost: 8084 }],
    env: [{ key: "PASSWORD", generate: true, label: "Mot de passe base de données" }],
    volumes: [],
    compose: `services:
  {{NAME}}:
    image: wordpress:latest
    ports:
      - "{{PORT}}:80"
    environment:
      WORDPRESS_DB_HOST: db
      WORDPRESS_DB_USER: wordpress
      WORDPRESS_DB_PASSWORD: {{PASSWORD}}
      WORDPRESS_DB_NAME: wordpress
    depends_on:
      - db
    volumes:
      - {{NAME}}-data:/var/www/html
    restart: unless-stopped
  db:
    image: mariadb:11
    environment:
      MYSQL_DATABASE: wordpress
      MYSQL_USER: wordpress
      MYSQL_PASSWORD: {{PASSWORD}}
      MYSQL_RANDOM_ROOT_PASSWORD: "1"
    volumes:
      - {{NAME}}-db:/var/lib/mysql
    restart: unless-stopped
volumes:
  {{NAME}}-data:
  {{NAME}}-db:
`,
  },
  {
    id: "vaultwarden",
    name: "Vaultwarden",
    mono: "Vw",
    color: "#175ddc",
    category: "Sécurité",
    description: "Gestionnaire de mots de passe (compatible Bitwarden).",
    image: "vaultwarden/server:latest",
    ports: [{ container: 80, defaultHost: 8083 }],
    env: [],
    volumes: ["{name}-data:/data"],
  },
];

/** Génère un mot de passe aléatoire sûr. */
export function genPassword(len = 20): string {
  const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
  const arr = new Uint32Array(len);
  crypto.getRandomValues(arr);
  return Array.from(arr, (n) => alphabet[n % alphabet.length]).join("");
}
