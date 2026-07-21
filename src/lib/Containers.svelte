<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { fly, slide } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "$lib/Icon.svelte";
  import { dockerAction, dockerList, dockerLogs } from "$lib/api";
  import type { Container, DockerStatus } from "$lib/types";

  let { profileId, password }: { profileId: string; password?: string } = $props();

  let status = $state<DockerStatus | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let actingId = $state<string | null>(null);

  let logsFor = $state<string | null>(null);
  let logsText = $state("");
  let logsLoading = $state(false);

  let timer: ReturnType<typeof setInterval> | null = null;

  const running = $derived(status?.containers.filter((c) => c.state === "running").length ?? 0);

  async function load() {
    try {
      status = await dockerList(profileId, password);
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function act(c: Container, action: "start" | "stop" | "restart") {
    actingId = c.id;
    error = null;
    try {
      await dockerAction(profileId, c.id, action, password);
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      actingId = null;
    }
  }

  async function toggleLogs(c: Container) {
    if (logsFor === c.id) {
      logsFor = null;
      return;
    }
    logsFor = c.id;
    logsText = "";
    logsLoading = true;
    try {
      logsText = (await dockerLogs(profileId, c.id, 200, password)) || "(aucune sortie)";
    } catch (e) {
      logsText = String(e);
    } finally {
      logsLoading = false;
    }
  }

  onMount(() => {
    load();
    timer = setInterval(load, 10000);
  });
  onDestroy(() => {
    if (timer) clearInterval(timer);
  });
</script>

<section class="panel" in:fly={{ y: 14, duration: 360, easing: quintOut }}>
  <div class="head">
    <h2>Conteneurs {#if status?.installed && status.containers.length}<span class="count">{running}/{status.containers.length} actifs</span>{/if}</h2>
    <button class="icon-btn" title="Rafraîchir" onclick={load} disabled={loading}>
      <Icon name="refresh" size={17} spin={loading && !status} />
    </button>
  </div>

  {#if loading && !status}
    <div class="state"><Icon name="spinner" size={22} spin /> Lecture de Docker…</div>
  {:else if error && !status}
    <div class="state err"><Icon name="alert" size={18} /> {error}</div>
  {:else if status && !status.installed}
    <div class="state">
      <Icon name="server" size={26} />
      <p>Docker n'est pas installé sur ce serveur.</p>
      <span class="muted">Le déploiement en 1 clic (App Store) arrive au prochain jalon.</span>
    </div>
  {:else if status && status.containers.length === 0}
    <div class="state"><Icon name="server" size={26} /><p>Aucun conteneur.</p></div>
  {:else if status}
    <ul class="list">
      {#each status.containers as c (c.id)}
        <li class="ctr" in:fly={{ y: 10, duration: 240, easing: quintOut }}>
          <div class="ctr-row">
            <span class="dot" class:on={c.state === "running"}></span>
            <div class="ctr-info">
              <strong>{c.name}</strong>
              <span class="sub">{c.image}</span>
              <span class="meta">
                {c.status}{#if c.state === "running" && c.cpuPercent != null}
                  · CPU {c.cpuPercent.toFixed(0)}%{#if c.memUsage} · {c.memUsage.split(" / ")[0]}{/if}
                {/if}
              </span>
            </div>
            <div class="ctr-actions">
              {#if c.state === "running"}
                <button class="act" title="Redémarrer" onclick={() => act(c, "restart")} disabled={actingId === c.id}>
                  <Icon name={actingId === c.id ? "spinner" : "refresh"} size={16} spin={actingId === c.id} />
                </button>
                <button class="act" title="Arrêter" onclick={() => act(c, "stop")} disabled={actingId === c.id}>
                  <Icon name="stop" size={16} />
                </button>
              {:else}
                <button class="act start" title="Démarrer" onclick={() => act(c, "start")} disabled={actingId === c.id}>
                  <Icon name={actingId === c.id ? "spinner" : "play"} size={16} spin={actingId === c.id} />
                </button>
              {/if}
              <button class="act" class:active={logsFor === c.id} title="Logs" onclick={() => toggleLogs(c)}>
                <Icon name="logs" size={16} />
              </button>
            </div>
          </div>

          {#if logsFor === c.id}
            <div class="logs" transition:slide={{ duration: 220 }}>
              {#if logsLoading}
                <span class="muted"><Icon name="spinner" size={13} spin /> Chargement des logs…</span>
              {:else}
                <pre>{logsText}</pre>
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
    {#if error}<div class="state err small"><Icon name="alert" size={14} /> {error}</div>{/if}
  {/if}
</section>

<style>
  .panel {
    padding: 1.4rem;
    border-radius: 18px;
    background: rgba(22, 22, 24, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(22px) saturate(1.1);
    -webkit-backdrop-filter: blur(22px) saturate(1.1);
    margin-bottom: 1.4rem;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
  }
  .count {
    font-size: 0.78rem;
    color: rgba(255, 255, 255, 0.45);
    font-weight: 400;
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: none;
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.06);
    color: #cdd6e6;
    cursor: pointer;
    transition: background 0.15s;
  }
  .icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 1.8rem 1rem;
    color: rgba(255, 255, 255, 0.6);
    text-align: center;
  }
  .state p {
    margin: 0;
  }
  .state.err {
    color: #f7a8a8;
    flex-direction: row;
    justify-content: center;
  }
  .state.small {
    padding: 0.6rem;
    font-size: 0.8rem;
  }
  .muted {
    color: rgba(255, 255, 255, 0.4);
    font-size: 0.8rem;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .ctr {
    background: rgba(255, 255, 255, 0.035);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 13px;
    padding: 0.8rem 0.9rem;
  }
  .ctr-row {
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: #5b6068;
    flex-shrink: 0;
  }
  .dot.on {
    background: #2dd4bf;
    box-shadow: 0 0 0 3px rgba(45, 212, 191, 0.18);
  }
  .ctr-info {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
    flex: 1;
  }
  .ctr-info strong {
    font-size: 0.95rem;
  }
  .sub {
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.78rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    color: rgba(255, 255, 255, 0.38);
    font-size: 0.74rem;
  }
  .ctr-actions {
    display: flex;
    gap: 0.35rem;
  }
  .act {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.05);
    color: #e7ecf3;
    cursor: pointer;
    transition: background 0.15s, transform 0.12s;
  }
  .act:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.13);
  }
  .act:active:not(:disabled) {
    transform: scale(0.94);
  }
  .act:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .act.start {
    color: #2dd4bf;
    border-color: rgba(45, 212, 191, 0.4);
  }
  .act.active {
    background: rgba(255, 255, 255, 0.14);
  }

  .logs {
    margin-top: 0.7rem;
  }
  .logs pre {
    margin: 0;
    max-height: 220px;
    overflow: auto;
    padding: 0.7rem 0.8rem;
    border-radius: 10px;
    background: rgba(0, 0, 0, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.08);
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 0.74rem;
    color: rgba(255, 255, 255, 0.8);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
