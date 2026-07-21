<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly, scale, slide } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "$lib/Icon.svelte";
  import { applyRoutes, caddyStatus, checkRoutes, dockerList, installCaddy } from "$lib/api";
  import type { CaddyRoute, Container, RouteHealth, SslMode } from "$lib/types";

  let {
    profileId,
    password,
    onClose,
  }: { profileId: string; password?: string; onClose: () => void } = $props();

  type Route = { id: string; domain: string; container: string; targetPort: number; ssl: SslMode };

  let installed = $state<boolean | null>(null);
  let installing = $state(false);
  let installErr = $state<string | null>(null);

  let containers = $state<Container[]>([]);
  let routes = $state<Route[]>([]);
  let health = $state<Record<string, RouteHealth>>({});
  let applying = $state(false);
  let applyErr = $state<string | null>(null);
  let checking = $state(false);

  // Formulaire d'ajout
  let fDomain = $state("");
  let fContainer = $state("");
  let fPort = $state<number | null>(null);
  let fSsl = $state<SslMode>("public");

  const RK = $derived(`beacon.routes.${profileId}`);

  function toCaddy(r: Route): CaddyRoute {
    return { domain: r.domain, targetPort: r.targetPort, ssl: r.ssl };
  }

  function persist() {
    try {
      localStorage.setItem(RK, JSON.stringify(routes));
    } catch {
      /* ignore */
    }
  }

  function parsePort(ports: string): number | null {
    const m = ports.match(/:(\d+)->/);
    return m ? Number(m[1]) : null;
  }

  function onPickContainer() {
    const c = containers.find((x) => x.name === fContainer);
    if (c) {
      const p = parsePort(c.ports);
      if (p) fPort = p;
    }
  }

  async function apply() {
    applying = true;
    applyErr = null;
    try {
      await applyRoutes(profileId, routes.map(toCaddy), password);
    } catch (e) {
      applyErr = String(e);
    } finally {
      applying = false;
    }
  }

  async function check() {
    if (routes.length === 0) {
      health = {};
      return;
    }
    checking = true;
    try {
      const res = await checkRoutes(profileId, routes.map(toCaddy), password);
      const map: Record<string, RouteHealth> = {};
      for (const h of res) map[h.domain] = h;
      health = map;
    } catch {
      /* garde l'état précédent */
    } finally {
      checking = false;
    }
  }

  async function addRoute(e: Event) {
    e.preventDefault();
    const domain = fDomain.trim().toLowerCase();
    if (!domain || !fPort) return;
    const id = crypto.randomUUID?.() ?? `r-${domain}`;
    routes = [...routes, { id, domain, container: fContainer, targetPort: fPort, ssl: fSsl }];
    persist();
    fDomain = "";
    fContainer = "";
    fPort = null;
    fSsl = "public";
    await apply();
    await check();
  }

  async function removeRoute(id: string) {
    routes = routes.filter((r) => r.id !== id);
    persist();
    await apply();
    await check();
  }

  async function installCaddyNow() {
    installing = true;
    installErr = null;
    try {
      await installCaddy(profileId, password);
      installed = true;
    } catch (e) {
      installErr = String(e);
    } finally {
      installing = false;
    }
  }

  function statusOf(r: Route): { kind: "ok" | "error" | "pending"; msg: string } {
    const h = health[r.domain];
    if (!h) return { kind: "pending", msg: "Non vérifié" };
    if (!h.portOk) return { kind: "error", msg: `Rien n'écoute sur le port ${r.targetPort}.` };
    if (r.ssl === "public" && !h.dnsOk) {
      return {
        kind: "error",
        msg: `Le domaine ne pointe pas vers ce serveur${h.serverIp ? ` (attendu ${h.serverIp})` : ""}.`,
      };
    }
    return { kind: "ok", msg: "En ligne" };
  }

  const cableColor = { ok: "#4ade80", error: "#f87171", pending: "#eab308" };

  onMount(async () => {
    try {
      routes = JSON.parse(localStorage.getItem(RK) || "[]");
    } catch {
      routes = [];
    }
    installed = await caddyStatus(profileId, password).catch(() => false);
    try {
      const s = await dockerList(profileId, password);
      containers = s.containers;
    } catch {
      containers = [];
    }
    check();
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="overlay"
  role="presentation"
  transition:fade={{ duration: 150 }}
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
>
  <div class="net" role="dialog" aria-modal="true" transition:scale={{ duration: 200, start: 0.96, easing: quintOut }}>
    <div class="net-head">
      <span class="net-icon"><Icon name="link" size={20} /></span>
      <h2>Réseau — Reverse proxy</h2>
      <button class="icon-btn" onclick={onClose}><Icon name="close" size={18} /></button>
    </div>

    {#if installed === null}
      <div class="state"><Icon name="spinner" size={22} spin /> Vérification de Caddy…</div>
    {:else if !installed}
      <div class="install">
        <div class="install-icon"><Icon name="link" size={30} /></div>
        <p class="install-title">Caddy est requis pour le reverse proxy</p>
        <p class="muted">
          Caddy gère le HTTPS automatiquement (Let's Encrypt) et les certificats locaux. Beacon
          peut l'installer via le dépôt officiel.
        </p>
        {#if installErr}<div class="err">{installErr}</div>{/if}
        <button class="primary" onclick={installCaddyNow} disabled={installing}>
          {#if installing}<Icon name="spinner" size={16} spin /> Installation…{:else}Installer Caddy{/if}
        </button>
      </div>
    {:else}
      {#if applyErr}<div class="err">{applyErr}</div>{/if}

      {#if routes.length === 0}
        <div class="state">
          <Icon name="link" size={26} />
          <p>Aucune liaison.</p>
          <span class="muted">Relie un domaine à un conteneur ci-dessous.</span>
        </div>
      {:else}
        <div class="routes">
          {#each routes as r (r.id)}
            {@const st = statusOf(r)}
            <div class="route" transition:slide={{ duration: 200 }}>
              <div class="node domain">
                <Icon name="link" size={15} />
                <div><strong>{r.domain}</strong><span>{r.ssl === "public" ? "HTTPS" : r.ssl === "local" ? "Local" : "HTTP"}</span></div>
              </div>

              <div class="cable" title={st.msg}>
                <svg viewBox="0 0 100 24" preserveAspectRatio="none">
                  <path d="M0,12 C35,12 65,12 100,12" fill="none" stroke={cableColor[st.kind]} stroke-width="2.5" stroke-linecap="round" class:flow={st.kind === "ok"} />
                </svg>
                <span class="cable-dot" style="background:{cableColor[st.kind]}"></span>
              </div>

              <div class="node target">
                <Icon name="server" size={15} />
                <div><strong>{r.container || "127.0.0.1"}</strong><span>port {r.targetPort}</span></div>
              </div>

              <button class="icon-btn danger" title="Supprimer" onclick={() => removeRoute(r.id)}><Icon name="trash" size={15} /></button>

              {#if st.kind === "error"}
                <div class="route-msg err"><Icon name="alert" size={13} /> {st.msg}</div>
              {:else if st.kind === "ok"}
                <div class="route-msg ok"><Icon name="check" size={13} /> {st.msg}</div>
              {/if}
            </div>
          {/each}
        </div>

        <button class="ghost check" onclick={check} disabled={checking}>
          {#if checking}<Icon name="spinner" size={15} spin /> Diagnostic…{:else}<Icon name="refresh" size={15} /> Vérifier les liaisons{/if}
        </button>
      {/if}

      <form class="add" onsubmit={addRoute}>
        <div class="add-title">Nouvelle liaison</div>
        <div class="add-grid">
          <label class="span2">
            <span>Domaine</span>
            <input bind:value={fDomain} placeholder="app.mondomaine.com" autocomplete="off" />
          </label>
          <label>
            <span>Conteneur <em>(optionnel)</em></span>
            <select bind:value={fContainer} onchange={onPickContainer}>
              <option value="">—</option>
              {#each containers as c (c.id)}<option value={c.name}>{c.name}</option>{/each}
            </select>
          </label>
          <label>
            <span>Port hôte</span>
            <input type="number" bind:value={fPort} min="1" max="65535" placeholder="8080" />
          </label>
          <label>
            <span>SSL</span>
            <select bind:value={fSsl}>
              <option value="public">Public (Let's Encrypt)</option>
              <option value="local">Local (cert interne)</option>
              <option value="none">HTTP simple</option>
            </select>
          </label>
        </div>
        <button class="primary" type="submit" disabled={applying || !fDomain.trim() || !fPort}>
          {#if applying}<Icon name="spinner" size={16} spin /> Application…{:else}<Icon name="plus" size={16} /> Relier{/if}
        </button>
      </form>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 1.5rem;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(5px);
    -webkit-backdrop-filter: blur(5px);
  }
  .net {
    width: 100%;
    max-width: 720px;
    max-height: 86vh;
    overflow-y: auto;
    padding: 1.4rem;
    border-radius: 20px;
    background: rgba(24, 24, 27, 0.94);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.65);
    color: #e7ecf3;
  }
  .net-head {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    margin-bottom: 1.2rem;
  }
  .net-head h2 {
    margin: 0;
    font-size: 1.2rem;
    flex: 1;
  }
  .net-icon {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: rgba(139, 147, 255, 0.2);
    color: #a5b4fc;
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border: none;
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.06);
    color: #cdd6e6;
    cursor: pointer;
  }
  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.12);
  }
  .icon-btn.danger:hover {
    background: rgba(220, 38, 38, 0.18);
    color: #fca5a5;
  }

  .muted {
    color: rgba(255, 255, 255, 0.45);
    font-size: 0.82rem;
  }
  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    justify-content: center;
    padding: 2.4rem 1rem;
    color: rgba(255, 255, 255, 0.6);
    text-align: center;
  }
  .state p {
    margin: 0;
  }

  .install {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.6rem;
    padding: 2rem 1rem;
  }
  .install-icon {
    display: grid;
    place-items: center;
    width: 60px;
    height: 60px;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.06);
    color: #fff;
  }
  .install-title {
    margin: 0.4rem 0 0;
    font-weight: 600;
    font-size: 1.05rem;
  }
  .install .muted {
    max-width: 440px;
  }

  .routes {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    margin-bottom: 0.8rem;
  }
  .route {
    display: grid;
    grid-template-columns: 1fr 90px 1fr auto;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem 0.8rem;
    border-radius: 13px;
    background: rgba(255, 255, 255, 0.035);
    border: 1px solid rgba(255, 255, 255, 0.07);
  }
  .node {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }
  .node div {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .node strong {
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .node span {
    font-size: 0.72rem;
    color: rgba(255, 255, 255, 0.4);
  }
  .node.target {
    justify-content: flex-end;
    text-align: right;
  }
  .node.target div {
    align-items: flex-end;
  }

  .cable {
    position: relative;
    height: 24px;
  }
  .cable svg {
    width: 100%;
    height: 100%;
  }
  .cable .flow {
    stroke-dasharray: 6 6;
    animation: flow 0.8s linear infinite;
  }
  @keyframes flow {
    to {
      stroke-dashoffset: -12;
    }
  }
  .cable-dot {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    box-shadow: 0 0 0 3px rgba(0, 0, 0, 0.3);
  }

  .route-msg {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.76rem;
  }
  .route-msg.err {
    color: #fca5a5;
  }
  .route-msg.ok {
    color: #86efac;
  }

  .check {
    margin-bottom: 1.2rem;
  }

  .add {
    padding: 1rem;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  .add-title {
    font-weight: 600;
    font-size: 0.95rem;
  }
  .add-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.7rem;
  }
  .span2 {
    grid-column: 1 / -1;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.6);
  }
  label em {
    color: rgba(255, 255, 255, 0.35);
    font-style: normal;
  }
  input,
  select {
    padding: 0.55rem 0.75rem;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(0, 0, 0, 0.4);
    color: #f4f4f5;
    font-size: 0.9rem;
    outline: none;
  }
  input:focus,
  select:focus {
    border-color: rgba(255, 255, 255, 0.4);
  }

  .primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    padding: 0.7rem;
    border: none;
    border-radius: 11px;
    background: #fff;
    color: #000;
    font-size: 0.92rem;
    font-weight: 600;
    cursor: pointer;
  }
  .primary:hover:not(:disabled) {
    filter: brightness(0.9);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .ghost {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.55rem 0.9rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.05);
    color: #e7ecf3;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .ghost:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }

  .err {
    padding: 0.7rem 0.9rem;
    border-radius: 11px;
    background: rgba(255, 90, 90, 0.12);
    border: 1px solid rgba(255, 120, 120, 0.32);
    color: #fecaca;
    font-size: 0.84rem;
    margin-bottom: 0.8rem;
    word-break: break-word;
  }
</style>
