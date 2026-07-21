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
</script>

<section class="panel">
  <div class="dash-head">
    <button class="btn ghost sm" onclick={onBack} title="Retour">
      <Icon name="arrow" size={18} />
    </button>
    <div class="dash-title">
      <strong>{metrics?.hostname || profile.label}</strong>
      <span class="sub">{profile.username}@{profile.host}:{profile.port}</span>
    </div>
    <span class="live" class:on={!error && !loading}>
      <span class="dot"></span>{error ? "hors ligne" : loading ? "…" : "en direct"}
    </span>
  </div>

  {#if loading && !metrics}
    <div class="loading"><Icon name="spinner" size={26} spin /> Récupération des métriques…</div>
  {:else if error && !metrics}
    <div class="feedback err">
      <span class="fb-title"><Icon name="alert" size={16} /> Connexion impossible</span>
      <span>{error}</span>
    </div>
  {:else if metrics}
    <div class="gauges">
      <Gauge value={metrics.cpuPercent} label="CPU" sublabel="{metrics.cpuCores} cœurs" />
      <Gauge
        value={memPercent}
        label="RAM"
        sublabel="{fmtBytes((metrics.memTotalKb - metrics.memAvailableKb) * 1024)} / {fmtBytes(
          metrics.memTotalKb * 1024,
        )}"
      />
      <Gauge
        value={diskPercent}
        label="Disque"
        sublabel="{fmtBytes(metrics.diskUsedBytes)} / {fmtBytes(metrics.diskTotalBytes)}"
      />
    </div>

    <div class="stats">
      <div class="stat">
        <span class="stat-label">Réseau</span>
        <div class="net">
          <span class="down"><Icon name="arrow" size={13} /> {fmtBytes(rxRate)}/s</span>
          <span class="up"><Icon name="arrow" size={13} /> {fmtBytes(txRate)}/s</span>
        </div>
      </div>
      <div class="stat">
        <span class="stat-label">Charge (1 / 5 / 15 min)</span>
        <strong class="load">
          {metrics.loadAvg[0].toFixed(2)} · {metrics.loadAvg[1].toFixed(2)} · {metrics.loadAvg[2].toFixed(2)}
        </strong>
      </div>
      <div class="stat">
        <span class="stat-label">Uptime</span>
        <strong>{fmtUptime(metrics.uptimeSecs)}</strong>
      </div>
    </div>

    {#if error}
      <div class="reconnect"><Icon name="spinner" size={13} spin /> Reconnexion… ({error})</div>
    {/if}
  {/if}
</section>

<style>
  .panel {
    background: rgba(20, 27, 44, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 18px;
    padding: 1.4rem;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(12px);
  }
  .dash-head {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    margin-bottom: 1.4rem;
  }
  .dash-title {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    flex: 1;
    min-width: 0;
  }
  .dash-title strong {
    font-size: 1.05rem;
  }
  .sub {
    color: #93a1bd;
    font-size: 0.8rem;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.76rem;
    color: #8695b3;
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

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    padding: 3rem 1rem;
    color: #93a1bd;
  }

  .gauges {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1rem;
    margin-bottom: 1.2rem;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.75rem;
  }
  .stat {
    background: rgba(9, 13, 24, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    padding: 0.8rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .stat-label {
    font-size: 0.72rem;
    color: #7d88a3;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .stat strong {
    font-size: 0.95rem;
  }
  .load {
    font-family: ui-monospace, monospace;
  }
  .net {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.86rem;
  }
  .net .down {
    color: #7ab0ff;
  }
  .net .up {
    color: #c084fc;
  }
  .net .up :global(svg) {
    transform: rotate(-90deg);
  }
  .net .down :global(svg) {
    transform: rotate(90deg);
  }

  .feedback {
    padding: 0.85rem 1rem;
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.86rem;
  }
  .feedback.err {
    background: rgba(127, 29, 29, 0.22);
    border: 1px solid rgba(248, 113, 113, 0.32);
  }
  .fb-title {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-weight: 600;
  }
  .reconnect {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.9rem;
    font-size: 0.76rem;
    color: #d9a441;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.5rem 0.7rem;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.05);
    color: #e7ecf3;
    cursor: pointer;
    transition: filter 0.15s;
  }
  .btn.ghost {
    background: transparent;
  }
  .btn :global(svg) {
    transform: rotate(180deg);
  }
  .btn:hover {
    filter: brightness(1.15);
  }
</style>
