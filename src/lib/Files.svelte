<script lang="ts">
  import { onMount } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "$lib/Icon.svelte";
  import { listDir } from "$lib/api";
  import type { DirListing } from "$lib/types";

  let {
    profileId,
    password,
    onClose,
  }: { profileId: string; password?: string; onClose: () => void } = $props();

  let listing = $state<DirListing | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function navigate(path: string) {
    loading = true;
    error = null;
    try {
      listing = await listDir(profileId, path, password);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function parentOf(p: string): string {
    if (p === "/" || p === "") return "/";
    const parts = p.replace(/\/+$/, "").split("/");
    parts.pop();
    const par = parts.join("/");
    return par === "" ? "/" : par;
  }

  function openEntry(name: string, isDir: boolean) {
    if (!isDir || !listing) return;
    const base = listing.path === "/" ? "" : listing.path.replace(/\/$/, "");
    navigate(`${base}/${name}`);
  }

  onMount(() => navigate("~"));
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
  <div class="files" role="dialog" aria-modal="true" transition:scale={{ duration: 200, start: 0.96, easing: quintOut }}>
    <div class="fhead">
      <span class="ficon"><Icon name="folder" size={20} /></span>
      <h2>Fichiers</h2>
      <button class="icon-btn" onclick={onClose}><Icon name="close" size={18} /></button>
    </div>

    <div class="bar">
      <button class="icon-btn" title="Accueil" onclick={() => navigate("~")}><Icon name="server" size={16} /></button>
      <button
        class="icon-btn"
        title="Dossier parent"
        onclick={() => listing && navigate(parentOf(listing.path))}
        disabled={!listing || listing.path === "/"}
      >
        <Icon name="arrow" size={16} />
      </button>
      <span class="path">{listing?.path ?? "…"}</span>
      <button class="icon-btn" title="Rafraîchir" onclick={() => listing && navigate(listing.path)} disabled={loading}>
        <Icon name="refresh" size={16} spin={loading} />
      </button>
    </div>

    {#if loading && !listing}
      <div class="state"><Icon name="spinner" size={22} spin /> Lecture…</div>
    {:else if error}
      <div class="state err"><Icon name="alert" size={18} /> {error}</div>
    {:else if listing}
      {#if listing.entries.length === 0}
        <div class="state"><Icon name="folder" size={24} /><p>Dossier vide.</p></div>
      {:else}
        <ul class="list">
          {#each listing.entries as e (e.name)}
            <li>
              <button class="entry" class:dir={e.isDir} onclick={() => openEntry(e.name, e.isDir)}>
                <Icon name={e.isDir ? "folder" : "logs"} size={18} />
                <span>{e.name}</span>
                {#if e.isDir}<span class="chev"><Icon name="arrow" size={14} /></span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}

    <p class="note"><Icon name="lock" size={12} /> Lecture seule pour l'instant — l'envoi/téléchargement arrivera plus tard.</p>
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
  .files {
    width: 100%;
    max-width: 620px;
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
  .fhead {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    margin-bottom: 1rem;
  }
  .fhead h2 {
    margin: 0;
    font-size: 1.2rem;
    flex: 1;
  }
  .ficon {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: rgba(56, 189, 248, 0.2);
    color: #7dd3fc;
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
    flex-shrink: 0;
  }
  .icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }
  .icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.5rem 0.6rem;
    border-radius: 11px;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.08);
    margin-bottom: 0.9rem;
  }
  .path {
    flex: 1;
    min-width: 0;
    font-family: ui-monospace, monospace;
    font-size: 0.82rem;
    color: rgba(255, 255, 255, 0.7);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 2.4rem 1rem;
    color: rgba(255, 255, 255, 0.6);
  }
  .state p {
    margin: 0;
  }
  .state.err {
    color: #f7a8a8;
    flex-direction: row;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    flex: 1;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.55rem 0.7rem;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: #dbe2ee;
    font-size: 0.88rem;
    text-align: left;
    cursor: pointer;
  }
  .entry:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .entry.dir {
    color: #fff;
  }
  .entry span:first-of-type {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .entry:not(.dir) :global(svg) {
    color: rgba(255, 255, 255, 0.4);
  }
  .chev {
    color: rgba(255, 255, 255, 0.3);
  }

  .note {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    margin: 0.9rem 0 0;
    font-size: 0.74rem;
    color: rgba(255, 255, 255, 0.4);
  }
</style>
