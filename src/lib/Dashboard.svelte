<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Gauge from "$lib/Gauge.svelte";
  import Icon from "$lib/Icon.svelte";
  import HddImage from "$lib/HddImage.svelte";
  import AppIcon from "$lib/AppIcon.svelte";
  import Containers from "$lib/Containers.svelte";
  import { fetchMetrics } from "$lib/api";
  import type { Metrics, ProfileMeta } from "$lib/types";

  let {
    profile,
    password,
    onBack,
  }: { profile: ProfileMeta; password?: string; onBack: () => void } = $props();

  let metrics = $state<Metrics | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

  let prevNet: { rx: number; tx: number; t: number } | null = null;
  let rxRate = $state(0);
  let txRate = $state(0);
  let netUp = $state<number[]>([]);
  let netDown = $state<number[]>([]);

  let clock = $state(new Date());

  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let clockTimer: ReturnType<typeof setInterval> | null = null;
  const POLL_MS = 4000;

  const memUsedKb = $derived(metrics ? metrics.memTotalKb - metrics.memAvailableKb : 0);
  const memPercent = $derived(
    metrics && metrics.memTotalKb > 0 ? (100 * memUsedKb) / metrics.memTotalKb : 0,
  );
  const diskPercent = $derived(
    metrics && metrics.diskTotalBytes > 0
      ? (100 * metrics.diskUsedBytes) / metrics.diskTotalBytes
      : 0,
  );

  // Courbe lisse (spline Catmull-Rom -> Bézier) : renvoie ligne + aire remplie.
  function smooth(vals: number[], max: number): { line: string; area: string } {
    const n = vals.length;
    if (n < 2) return { line: "", area: "" };
    const step = 100 / (n - 1);
    const p = vals.map((v, i) => [i * step, 28 - (v / Math.max(max, 1)) * 25] as [number, number]);
    let d = `M${p[0][0].toFixed(2)},${p[0][1].toFixed(2)}`;
    for (let i = 0; i < n - 1; i++) {
      const p0 = p[i - 1] ?? p[i];
      const p1 = p[i];
      const p2 = p[i + 1];
      const p3 = p[i + 2] ?? p2;
      const c1x = p1[0] + (p2[0] - p0[0]) / 6;
      const c1y = p1[1] + (p2[1] - p0[1]) / 6;
      const c2x = p2[0] - (p3[0] - p1[0]) / 6;
      const c2y = p2[1] - (p3[1] - p1[1]) / 6;
      d += ` C${c1x.toFixed(2)},${c1y.toFixed(2)} ${c2x.toFixed(2)},${c2y.toFixed(2)} ${p2[0].toFixed(2)},${p2[1].toFixed(2)}`;
    }
    return { line: d, area: `${d} L100,30 L0,30 Z` };
  }

  const netMax = $derived(Math.max(1, ...netUp, ...netDown));
  const upCurve = $derived(smooth(netUp, netMax));
  const downCurve = $derived(smooth(netDown, netMax));

  function fmtBytes(b: number): string {
    const u = ["o", "Ko", "Mo", "Go", "To"];
    let i = 0;
    let v = b;
    while (v >= 1024 && i < u.length - 1) {
      v /= 1024;
      i++;
    }
    return `${v.toFixed(v < 10 && i > 0 ? 1 : 0)} ${u[i]}`;
  }
  function fmtRate(b: number): string {
    return b < 1024 ? `${Math.round(b)} o/s` : `${(b / 1024).toFixed(b < 1024 * 10 ? 1 : 0)} Ko/s`;
  }
  function fmtUptime(s: number): string {
    const d = Math.floor(s / 86400);
    const h = Math.floor((s % 86400) / 3600);
    const m = Math.floor((s % 3600) / 60);
    if (d > 0) return `${d} j ${h} h`;
    if (h > 0) return `${h} h ${m} min`;
    return `${m} min`;
  }

  async function poll() {
    try {
      const m = await fetchMetrics(profile.id, password);
      const now = Date.now();
      if (prevNet) {
        const dt = (now - prevNet.t) / 1000;
        if (dt > 0) {
          rxRate = Math.max(0, (m.netRxBytes - prevNet.rx) / dt);
          txRate = Math.max(0, (m.netTxBytes - prevNet.tx) / dt);
          netDown = [...netDown, rxRate].slice(-24);
          netUp = [...netUp, txRate].slice(-24);
        }
      }
      prevNet = { rx: m.netRxBytes, tx: m.netTxBytes, t: now };
      metrics = m;
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    poll();
    pollTimer = setInterval(poll, POLL_MS);
    clockTimer = setInterval(() => (clock = new Date()), 1000);
  });
  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
    if (clockTimer) clearInterval(clockTimer);
  });

  const APPS = [
    { key: "appstore", label: "App Store" },
    { key: "files", label: "Fichiers" },
    { key: "terminal", label: "Terminal" },
    { key: "containers", label: "Conteneurs" },
    { key: "network", label: "Réseau" },
    { key: "settings", label: "Réglages" },
  ];
</script>

<div class="zos">
  <!-- Colonne widgets -->
  <aside class="col-left">
    <div class="left-top">
      <span class="brand"><Icon name="beacon" size={20} /> Beacon</span>
      <button class="icon-btn" title="Déconnecter" onclick={onBack}><Icon name="logout" size={18} /></button>
    </div>

    <!-- Horloge -->
    <div class="w clock" in:fly={{ x: -16, duration: 420, easing: quintOut }}>
      <div class="time">{clock.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}</div>
      <div class="date">
        {clock.toLocaleDateString("fr-FR", { weekday: "long", day: "numeric", month: "long", year: "numeric" })}
      </div>
    </div>

    <!-- Système -->
    <div class="w" in:fly={{ x: -16, duration: 420, delay: 70, easing: quintOut }}>
      <div class="w-head"><span>Système</span></div>
      {#if metrics}
        <div class="sys-gauges">
          <div class="mini-gauge">
            <Gauge value={metrics.cpuPercent} label="CPU" />
            <span class="mini-sub">{metrics.cpuCores} cœurs</span>
          </div>
          <div class="mini-gauge">
            <Gauge value={memPercent} label="RAM" />
            <span class="mini-sub">{fmtBytes(metrics.memTotalKb * 1024)}</span>
          </div>
        </div>
      {:else}
        <div class="w-empty"><Icon name="spinner" size={20} spin /></div>
      {/if}
    </div>

    <!-- Stockage -->
    <div class="w" in:fly={{ x: -16, duration: 420, delay: 140, easing: quintOut }}>
      <div class="w-head"><span>Stockage</span></div>
      {#if metrics}
        <div class="storage">
          <div class="storage-top">
            <HddImage size={52} />
            <div class="storage-meta">
              <span class="badge-ok">Sain</span>
              <span class="storage-txt">
                {fmtBytes(metrics.diskUsedBytes)} <em>/ {fmtBytes(metrics.diskTotalBytes)}</em>
              </span>
            </div>
          </div>
          <div class="bar"><div class="bar-fill" style="width:{Math.min(100, diskPercent)}%"></div></div>
        </div>
      {:else}
        <div class="w-empty"><Icon name="spinner" size={20} spin /></div>
      {/if}
    </div>

    <!-- Réseau -->
    <div class="w" in:fly={{ x: -16, duration: 420, delay: 210, easing: quintOut }}>
      <div class="w-head"><span>Réseau</span><span class="iface">eth0</span></div>
      <svg class="spark" viewBox="0 0 100 30" preserveAspectRatio="none">
        <defs>
          <linearGradient id="net-down" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stop-color="#2dd4bf" stop-opacity="0.4" />
            <stop offset="1" stop-color="#2dd4bf" stop-opacity="0" />
          </linearGradient>
          <linearGradient id="net-up" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stop-color="#4ade80" stop-opacity="0.32" />
            <stop offset="1" stop-color="#4ade80" stop-opacity="0" />
          </linearGradient>
        </defs>
        <line x1="0" y1="10" x2="100" y2="10" class="grid" />
        <line x1="0" y1="20" x2="100" y2="20" class="grid" />
        {#if netDown.length > 1}
          <path d={downCurve.area} fill="url(#net-down)" />
          <path d={upCurve.area} fill="url(#net-up)" />
          <path d={downCurve.line} fill="none" stroke="#2dd4bf" stroke-width="1.4" />
          <path d={upCurve.line} fill="none" stroke="#4ade80" stroke-width="1.4" />
        {/if}
      </svg>
      <div class="net-rates">
        <span class="up"><Icon name="arrow" size={12} /> {fmtRate(txRate)}</span>
        <span class="down"><Icon name="arrow" size={12} /> {fmtRate(rxRate)}</span>
      </div>
    </div>

    <div class="left-foot">
      <span>{fmtUptime(metrics?.uptimeSecs ?? 0)} d'uptime</span>
    </div>
  </aside>

  <!-- Zone principale -->
  <main class="col-main">
    <div class="topbar">
      <div class="searchbar">
        <Icon name="search" size={18} />
        <input placeholder="Rechercher…" />
      </div>
      <span class="live" class:on={!error && !loading}>
        <span class="dot"></span>{error ? "hors ligne" : loading ? "…" : "en direct"}
      </span>
    </div>

    <div class="server-hero">
      <div class="hero-icon"><Icon name="server" size={26} /></div>
      <div class="hero-info">
        <strong>{profile.label}</strong>
        <span>
          {profile.username}@{profile.host}:{profile.port}{#if metrics?.hostname}
            <span class="hn" title="Hostname du serveur">· {metrics.hostname}</span>
          {/if}
        </span>
      </div>
      {#if error && metrics}
        <span class="hero-warn"><Icon name="spinner" size={13} spin /> reconnexion…</span>
      {/if}
    </div>

    {#if error && !metrics}
      <div class="fatal">
        <Icon name="alert" size={30} />
        <p>Connexion impossible</p>
        <span>{error}</span>
        <button class="btn" onclick={onBack}>Retour</button>
      </div>
    {:else}
      <section class="apps-panel" in:fly={{ y: 14, duration: 360, easing: quintOut }}>
        <div class="apps-head">
          <h2>Applications</h2>
          <button class="icon-btn" title="Ajouter (bientôt)" disabled><Icon name="plus" size={18} /></button>
        </div>
        <div class="apps">
          {#each APPS as app, i (app.label)}
            <button
              class="tile"
              type="button"
              title="Bientôt disponible"
              in:fly={{ y: 16, duration: 340, delay: i * 45, easing: quintOut }}
            >
              <span class="tile-icon"><AppIcon name={app.key} size={60} /></span>
              <span class="tile-label">{app.label}</span>
            </button>
          {/each}
        </div>
        <p class="apps-note">
          <Icon name="apps" size={14} /> Applications intégrées — l'App Store 1-clic arrive au prochain jalon.
        </p>
      </section>

      <Containers profileId={profile.id} {password} />
    {/if}
  </main>
</div>

<style>
  .zos {
    height: 100vh;
    width: 100%;
    display: grid;
    grid-template-columns: 300px 1fr;
    color: #f4f4f5;
    font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
    background: transparent;
  }

  /* Colonne gauche */
  .col-left {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    padding: 1rem;
    overflow-y: auto;
  }
  .left-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.1rem 0.3rem 0.3rem;
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 700;
    font-size: 1.02rem;
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.05);
    color: #aeb9d1;
    cursor: pointer;
  }
  .icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }
  .icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .w {
    background: rgba(22, 22, 24, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    padding: 0.9rem 1rem;
    backdrop-filter: blur(22px) saturate(1.1);
    -webkit-backdrop-filter: blur(22px) saturate(1.1);
  }
  .w-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: #aeb9d1;
    font-size: 0.82rem;
    margin-bottom: 0.7rem;
  }
  .w-empty {
    display: grid;
    place-items: center;
    padding: 1rem;
    color: #7d88a3;
  }
  .clock {
    padding: 1.1rem 1.1rem 1rem;
  }
  .time {
    font-size: 2.4rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1;
  }
  .date {
    color: #8695b3;
    font-size: 0.8rem;
    margin-top: 0.35rem;
    text-transform: capitalize;
  }

  .sys-gauges {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  .mini-gauge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.2rem;
  }
  .mini-gauge :global(svg) {
    max-width: 92px;
  }
  .mini-sub {
    font-size: 0.72rem;
    color: #8695b3;
  }

  .storage-top {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.7rem;
  }
  .storage-meta {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .badge-ok {
    align-self: flex-start;
    font-size: 0.72rem;
    color: #34d399;
    background: rgba(52, 211, 153, 0.15);
    padding: 0.15rem 0.5rem;
    border-radius: 6px;
    font-weight: 600;
  }
  .storage-txt {
    font-size: 0.86rem;
    color: #fff;
  }
  .storage-txt em {
    font-style: normal;
    color: rgba(255, 255, 255, 0.5);
  }
  .bar {
    height: 7px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.08);
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    border-radius: 4px;
    background: linear-gradient(90deg, rgba(255, 255, 255, 0.55), #fff);
    transition: width 0.6s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .spark {
    width: 100%;
    height: 42px;
    display: block;
  }
  .grid {
    stroke: rgba(255, 255, 255, 0.06);
    stroke-width: 0.5;
  }
  .iface {
    color: #7d88a3;
    font-size: 0.76rem;
  }
  .net-rates {
    display: flex;
    justify-content: flex-start;
    gap: 1.1rem;
    margin-top: 0.4rem;
    font-size: 0.78rem;
    color: #fff;
  }
  .net-rates .up,
  .net-rates .down {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }
  .net-rates .up :global(svg) {
    color: #4ade80;
    transform: rotate(-90deg);
  }
  .net-rates .down :global(svg) {
    color: #2dd4bf;
    transform: rotate(90deg);
  }
  .left-foot {
    margin-top: auto;
    padding: 0.4rem 0.3rem;
    color: #6f7b96;
    font-size: 0.74rem;
  }

  /* Zone principale */
  .col-main {
    padding: 1.3rem 1.8rem 2rem;
    overflow-y: auto;
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.3rem;
  }
  .searchbar {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem 1rem;
    border-radius: 12px;
    background: rgba(22, 22, 24, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #7d88a3;
    backdrop-filter: blur(20px) saturate(1.1);
    -webkit-backdrop-filter: blur(20px) saturate(1.1);
  }
  .searchbar input {
    flex: 1;
    border: none;
    background: transparent;
    color: #e7ecf3;
    font-size: 0.92rem;
    outline: none;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.8rem;
    color: #8695b3;
    padding: 0.45rem 0.85rem;
    border-radius: 20px;
    background: rgba(22, 22, 24, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.08);
    white-space: nowrap;
    backdrop-filter: blur(20px) saturate(1.1);
    -webkit-backdrop-filter: blur(20px) saturate(1.1);
  }
  .live .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #6b7280;
  }
  .live.on .dot {
    background: #4ade80;
    box-shadow: 0 0 0 3px rgba(74, 222, 128, 0.22);
  }

  .server-hero {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    padding: 1.1rem 1.2rem;
    border-radius: 16px;
    background: rgba(22, 22, 24, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.08);
    margin-bottom: 1.6rem;
    backdrop-filter: blur(22px) saturate(1.1);
    -webkit-backdrop-filter: blur(22px) saturate(1.1);
  }
  .hero-icon {
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }
  .hero-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .hero-info strong {
    font-size: 1.05rem;
  }
  .hero-info span {
    color: #8695b3;
    font-size: 0.82rem;
  }
  .hn {
    color: rgba(255, 255, 255, 0.32);
  }
  .hero-warn {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    color: #d9a441;
    font-size: 0.76rem;
  }

  .apps-panel {
    padding: 1.4rem;
    border-radius: 18px;
    background: rgba(22, 22, 24, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(22px) saturate(1.1);
    -webkit-backdrop-filter: blur(22px) saturate(1.1);
    margin-bottom: 1.4rem;
  }
  .apps-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.1rem;
  }
  .apps-head h2 {
    margin: 0;
    font-size: 1.1rem;
  }
  .apps {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 1.1rem;
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    border: none;
    background: transparent;
    color: #cdd6e6;
    cursor: pointer;
    padding: 0;
  }
  .tile-icon {
    display: grid;
    place-items: center;
    transition: transform 0.22s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .tile:hover .tile-icon {
    transform: translateY(-3px) scale(1.05);
  }
  .tile:active .tile-icon {
    transform: translateY(-1px) scale(0.99);
  }
  .tile-label {
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.7);
  }
  .apps-note {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    margin: 1.6rem 0 0;
    font-size: 0.8rem;
    color: #6f7b96;
  }

  .fatal {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.7rem;
    padding: 5rem 1rem;
    color: #93a1bd;
    text-align: center;
  }
  .fatal p {
    margin: 0;
    font-weight: 600;
    color: #f7a8a8;
  }
  .fatal span {
    font-size: 0.85rem;
    max-width: 420px;
  }
  .btn {
    padding: 0.6rem 1.1rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.06);
    color: #e7ecf3;
    cursor: pointer;
    font-size: 0.9rem;
  }
  .btn:hover {
    filter: brightness(1.15);
  }
</style>
