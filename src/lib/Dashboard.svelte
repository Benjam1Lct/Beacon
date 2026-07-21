<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import Gauge from "$lib/Gauge.svelte";
  import Icon from "$lib/Icon.svelte";
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

  let timer: ReturnType<typeof setInterval> | null = null;
  const POLL_MS = 4000;

  const memPercent = $derived(
    metrics && metrics.memTotalKb > 0
      ? (100 * (metrics.memTotalKb - metrics.memAvailableKb)) / metrics.memTotalKb
      : 0,
  );
  const diskPercent = $derived(
    metrics && metrics.diskTotalBytes > 0
      ? (100 * metrics.diskUsedBytes) / metrics.diskTotalBytes
      : 0,
  );

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
    timer = setInterval(poll, POLL_MS);
  });
  onDestroy(() => {
    if (timer) clearInterval(timer);
  });

  const NAV = [
    { icon: "grid", label: "Tableau de bord", active: true },
    { icon: "apps", label: "Applications", active: false },
    { icon: "link", label: "Réseau", active: false },
    { icon: "folder", label: "Fichiers", active: false },
    { icon: "settings", label: "Réglages", active: false },
  ];
</script>

<div class="os">
  <aside class="sidebar">
    <div class="brand">
      <span class="brand-icon"><Icon name="beacon" size={22} /></span>
      <span class="brand-name">Beacon</span>
    </div>

    <nav>
      {#each NAV as item (item.label)}
        <button class="nav" class:active={item.active} disabled={!item.active}>
          <Icon name={item.icon} size={19} />
          <span>{item.label}</span>
          {#if !item.active}<span class="soon">bientôt</span>{/if}
        </button>
      {/each}
    </nav>

    <div class="side-foot">
      <div class="server-chip">
        <span class="chip-dot" class:on={!error && !loading}></span>
        <div class="chip-info">
          <strong>{metrics?.hostname || profile.label}</strong>
          <span>{profile.host}</span>
        </div>
      </div>
      <button class="nav" onclick={onBack}>
        <Icon name="logout" size={19} />
        <span>Déconnecter</span>
      </button>
    </div>
  </aside>

  <main class="content">
    <header class="topbar">
      <div class="tb-title">
        <h1>{metrics?.hostname || profile.label}</h1>
        <span class="sub">{profile.username}@{profile.host}:{profile.port}</span>
      </div>
      <span class="live" class:on={!error && !loading}>
        <span class="dot"></span>{error ? "hors ligne" : loading ? "…" : "en direct"}
      </span>
    </header>

    {#if loading && !metrics}
      <div class="loading"><Icon name="spinner" size={28} spin /> Récupération des métriques…</div>
    {:else if error && !metrics}
      <div class="fatal">
        <Icon name="alert" size={30} />
        <p>Connexion impossible</p>
        <span>{error}</span>
        <button class="btn" onclick={onBack}>Retour</button>
      </div>
    {:else if metrics}
      <section class="section-title">Ressources</section>
      <div class="widgets">
        <div class="widget">
          <div class="w-head"><Icon name="memory" size={17} /> Processeur</div>
          <Gauge value={metrics.cpuPercent} label="CPU" sublabel="{metrics.cpuCores} cœurs" />
        </div>
        <div class="widget">
          <div class="w-head"><Icon name="memory" size={17} /> Mémoire</div>
          <Gauge
            value={memPercent}
            label="RAM"
            sublabel="{fmtBytes((metrics.memTotalKb - metrics.memAvailableKb) * 1024)} / {fmtBytes(
              metrics.memTotalKb * 1024,
            )}"
          />
        </div>
        <div class="widget">
          <div class="w-head"><Icon name="disk" size={17} /> Disque</div>
          <Gauge
            value={diskPercent}
            label="Disque"
            sublabel="{fmtBytes(metrics.diskUsedBytes)} / {fmtBytes(metrics.diskTotalBytes)}"
          />
        </div>

        <div class="widget wide">
          <div class="w-head"><Icon name="link" size={17} /> Réseau</div>
          <div class="net-big">
            <div class="net-item down">
              <Icon name="arrow" size={18} />
              <div><strong>{fmtBytes(rxRate)}/s</strong><span>Descendant</span></div>
            </div>
            <div class="net-item up">
              <Icon name="arrow" size={18} />
              <div><strong>{fmtBytes(txRate)}/s</strong><span>Montant</span></div>
            </div>
          </div>
        </div>

        <div class="widget wide">
          <div class="w-head"><Icon name="server" size={17} /> Système</div>
          <div class="sys">
            <div class="sys-row">
              <span>Charge (1 / 5 / 15 min)</span>
              <strong class="mono"
                >{metrics.loadAvg[0].toFixed(2)} · {metrics.loadAvg[1].toFixed(2)} · {metrics.loadAvg[2].toFixed(
                  2,
                )}</strong
              >
            </div>
            <div class="sys-row"><span>Uptime</span><strong>{fmtUptime(metrics.uptimeSecs)}</strong></div>
            <div class="sys-row"><span>Hôte</span><strong>{metrics.hostname}</strong></div>
          </div>
        </div>
      </div>

      <section class="section-title">Applications</section>
      <div class="apps-empty">
        <Icon name="apps" size={26} />
        <p>Le gestionnaire de containers / app store arrive au prochain jalon.</p>
      </div>

      {#if error}
        <div class="reconnect"><Icon name="spinner" size={13} spin /> Reconnexion… ({error})</div>
      {/if}
    {/if}
  </main>
</div>

<style>
  .os {
    height: 100vh;
    width: 100%;
    display: grid;
    grid-template-columns: 240px 1fr;
    color: #e7ecf3;
    font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
    background: radial-gradient(1200px 800px at 70% -10%, #16223d 0%, #0c1220 55%, #080b14 100%);
  }

  /* Sidebar */
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 1.1rem 0.8rem;
    background: rgba(10, 14, 24, 0.55);
    border-right: 1px solid rgba(255, 255, 255, 0.07);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.6rem 1rem;
  }
  .brand-icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 11px;
    background: linear-gradient(180deg, #3b82f6, #2563eb);
    color: white;
  }
  .brand-name {
    font-weight: 700;
    font-size: 1.1rem;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    flex: 1;
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.65rem 0.7rem;
    border: none;
    border-radius: 10px;
    background: transparent;
    color: #aeb9d1;
    font-size: 0.9rem;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .nav:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.05);
    color: #fff;
  }
  .nav.active {
    background: rgba(37, 99, 235, 0.2);
    color: #fff;
  }
  .nav:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .nav span:first-of-type {
    flex: 1;
  }
  .soon {
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #6f7b96;
    background: rgba(255, 255, 255, 0.06);
    padding: 0.1rem 0.35rem;
    border-radius: 6px;
  }
  .side-foot {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .server-chip {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.6rem 0.7rem;
    border-radius: 10px;
    background: rgba(9, 13, 24, 0.5);
  }
  .chip-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: #6b7280;
    flex-shrink: 0;
  }
  .chip-dot.on {
    background: #4ade80;
    box-shadow: 0 0 0 3px rgba(74, 222, 128, 0.2);
  }
  .chip-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .chip-info strong {
    font-size: 0.82rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chip-info span {
    font-size: 0.72rem;
    color: #7d88a3;
  }

  /* Content */
  .content {
    padding: 1.6rem 2rem 2.4rem;
    overflow-y: auto;
  }
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.6rem;
  }
  .tb-title h1 {
    margin: 0;
    font-size: 1.5rem;
    letter-spacing: -0.02em;
  }
  .tb-title .sub {
    color: #93a1bd;
    font-size: 0.85rem;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.8rem;
    color: #8695b3;
    padding: 0.4rem 0.8rem;
    border-radius: 20px;
    background: rgba(9, 13, 24, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.06);
  }
  .live .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #6b7280;
  }
  .live.on .dot {
    background: #4ade80;
    box-shadow: 0 0 0 3px rgba(74, 222, 128, 0.2);
  }

  .section-title {
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #7d88a3;
    margin: 0.4rem 0 0.9rem;
  }

  .widgets {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 1rem;
    margin-bottom: 1.8rem;
  }
  .widget {
    background: rgba(20, 27, 44, 0.62);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 16px;
    padding: 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    box-shadow: 0 12px 34px rgba(0, 0, 0, 0.28);
  }
  .widget.wide {
    grid-column: span 1;
    justify-content: flex-start;
  }
  .w-head {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.9rem;
    font-weight: 600;
    color: #cdd6e6;
  }

  .net-big {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    padding-top: 0.4rem;
  }
  .net-item {
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }
  .net-item div {
    display: flex;
    flex-direction: column;
  }
  .net-item strong {
    font-size: 1.1rem;
  }
  .net-item span {
    font-size: 0.74rem;
    color: #7d88a3;
  }
  .net-item.down {
    color: #7ab0ff;
  }
  .net-item.up {
    color: #c084fc;
  }
  .net-item.down :global(svg) {
    transform: rotate(90deg);
  }
  .net-item.up :global(svg) {
    transform: rotate(-90deg);
  }

  .sys {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding-top: 0.2rem;
  }
  .sys-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    font-size: 0.85rem;
  }
  .sys-row span {
    color: #93a1bd;
  }
  .mono {
    font-family: ui-monospace, monospace;
  }

  .apps-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    padding: 2.2rem 1rem;
    color: #7d88a3;
    text-align: center;
    background: rgba(20, 27, 44, 0.4);
    border: 1px dashed rgba(255, 255, 255, 0.1);
    border-radius: 16px;
  }
  .apps-empty p {
    margin: 0;
    font-size: 0.86rem;
  }

  .loading,
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
  .loading {
    flex-direction: row;
  }
  .fatal p {
    margin: 0;
    font-weight: 600;
    color: #f7a8a8;
  }
  .fatal span {
    font-size: 0.85rem;
    max-width: 400px;
  }

  .reconnect {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: 1rem;
    font-size: 0.76rem;
    color: #d9a441;
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
