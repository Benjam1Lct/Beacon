<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "$lib/Icon.svelte";
  import { listProcesses } from "$lib/api";
  import type { Process } from "$lib/types";

  let {
    profileId,
    password,
    onClose,
  }: { profileId: string; password?: string; onClose: () => void } = $props();

  let procs = $state<Process[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let query = $state("");
  let timer: ReturnType<typeof setInterval> | null = null;

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return procs;
    return procs.filter(
      (p) => p.command.toLowerCase().includes(q) || p.user.toLowerCase().includes(q) || String(p.pid).includes(q),
    );
  });

  const totalCpu = $derived(procs.reduce((s, p) => s + p.cpu, 0));
  const totalMem = $derived(procs.reduce((s, p) => s + p.mem, 0));

  async function load() {
    try {
      procs = await listProcesses(profileId, password);
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    load();
    timer = setInterval(load, 3000);
  });
  onDestroy(() => {
    if (timer) clearInterval(timer);
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
  <div class="pm" role="dialog" aria-modal="true" transition:scale={{ duration: 200, start: 0.96, easing: quintOut }}>
    <div class="pm-head">
      <span class="pm-icon"><Icon name="grid" size={20} /></span>
      <h2>Processus</h2>
      <span class="live" class:on={!loading && !error}><span class="dot"></span>{error ? "erreur" : "en direct"}</span>
      <button class="icon-btn" onclick={onClose}><Icon name="close" size={18} /></button>
    </div>

    <div class="pm-top">
      <div class="search">
        <Icon name="search" size={16} />
        <input bind:value={query} placeholder="Filtrer (nom, utilisateur, PID)…" autocomplete="off" />
      </div>
      <div class="totals">
        <span>CPU total <strong>{totalCpu.toFixed(0)}%</strong></span>
        <span>RAM totale <strong>{totalMem.toFixed(0)}%</strong></span>
        <span>{procs.length} process</span>
      </div>
    </div>

    {#if loading && procs.length === 0}
      <div class="state"><Icon name="spinner" size={22} spin /> Lecture des processus…</div>
    {:else if error && procs.length === 0}
      <div class="state err"><Icon name="alert" size={18} /> {error}</div>
    {:else}
      <div class="table">
        <div class="thead">
          <span class="c-pid">PID</span>
          <span class="c-user">Utilisateur</span>
          <span class="c-bar">CPU</span>
          <span class="c-bar">RAM</span>
          <span class="c-cmd">Commande</span>
        </div>
        <div class="tbody">
          {#each filtered as p (p.pid)}
            <div class="trow">
              <span class="c-pid">{p.pid}</span>
              <span class="c-user">{p.user}</span>
              <span class="c-bar">
                <span class="bar"><span class="fill cpu" style="width:{Math.min(100, p.cpu)}%"></span></span>
                <em>{p.cpu.toFixed(1)}</em>
              </span>
              <span class="c-bar">
                <span class="bar"><span class="fill mem" style="width:{Math.min(100, p.mem)}%"></span></span>
                <em>{p.mem.toFixed(1)}</em>
              </span>
              <span class="c-cmd" title={p.command}>{p.command}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 65;
    display: grid;
    place-items: center;
    padding: 1.5rem;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(5px);
    -webkit-backdrop-filter: blur(5px);
  }
  .pm {
    width: 100%;
    max-width: 760px;
    max-height: 86vh;
    display: flex;
    flex-direction: column;
    padding: 1.4rem;
    border-radius: 20px;
    background: rgba(24, 24, 27, 0.94);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.65);
    color: #e7ecf3;
  }
  .pm-head {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    margin-bottom: 1rem;
  }
  .pm-head h2 {
    margin: 0;
    font-size: 1.2rem;
    flex: 1;
  }
  .pm-icon {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: rgba(74, 222, 128, 0.18);
    color: #86efac;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.76rem;
    color: #8695b3;
    padding: 0.35rem 0.7rem;
    border-radius: 20px;
    background: rgba(255, 255, 255, 0.05);
  }
  .live .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #6b7280;
  }
  .live.on .dot {
    background: #4ade80;
    box-shadow: 0 0 0 3px rgba(74, 222, 128, 0.2);
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

  .pm-top {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    margin-bottom: 0.9rem;
    flex-wrap: wrap;
  }
  .search {
    flex: 1;
    min-width: 200px;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.55rem 0.8rem;
    border-radius: 11px;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.5);
  }
  .search input {
    flex: 1;
    border: none;
    background: transparent;
    color: #f4f4f5;
    font-size: 0.88rem;
    outline: none;
  }
  .totals {
    display: flex;
    gap: 1rem;
    font-size: 0.78rem;
    color: rgba(255, 255, 255, 0.45);
  }
  .totals strong {
    color: #fff;
  }

  .state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    padding: 3rem;
    color: rgba(255, 255, 255, 0.6);
  }
  .state.err {
    color: #f7a8a8;
  }

  .table {
    overflow: auto;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.07);
  }
  .thead,
  .trow {
    display: grid;
    grid-template-columns: 64px 110px 1fr 1fr 2.4fr;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.8rem;
  }
  .thead {
    position: sticky;
    top: 0;
    background: rgba(30, 30, 34, 0.98);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: rgba(255, 255, 255, 0.4);
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }
  .trow {
    font-size: 0.82rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }
  .trow:hover {
    background: rgba(255, 255, 255, 0.03);
  }
  .c-pid {
    color: rgba(255, 255, 255, 0.5);
    font-family: ui-monospace, monospace;
  }
  .c-user {
    color: rgba(255, 255, 255, 0.65);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .c-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .c-bar em {
    font-style: normal;
    font-size: 0.74rem;
    color: rgba(255, 255, 255, 0.6);
    min-width: 30px;
    text-align: right;
  }
  .bar {
    flex: 1;
    height: 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.08);
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    border-radius: 4px;
    transition: width 0.4s ease;
  }
  .fill.cpu {
    background: #4ade80;
  }
  .fill.mem {
    background: #2dd4bf;
  }
  .c-cmd {
    font-family: ui-monospace, "Cascadia Code", monospace;
    color: rgba(255, 255, 255, 0.85);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
