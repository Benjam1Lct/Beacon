<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { fade, fly, scale, slide } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "$lib/Icon.svelte";
  import { dockerAction, dockerList, dockerLogs } from "$lib/api";
  import type { Container, DockerStatus } from "$lib/types";

  let { profileId, password }: { profileId: string; password?: string } = $props();

  let status = $state<DockerStatus | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let acting = $state(false);

  // Détail (popover)
  let selected = $state<Container | null>(null);
  let logsOpen = $state(false);
  let logsText = $state("");
  let logsLoading = $state(false);

  // Dossiers (regroupement, persistés en local par serveur)
  type Folder = { id: string; name: string };
  let folders = $state<Folder[]>([]);
  let assign = $state<Record<string, string>>({});
  let newFolder = $state("");
  let creating = $state(false);

  const FK = $derived(`beacon.folders.${profileId}`);
  const AK = $derived(`beacon.assign.${profileId}`);

  let timer: ReturnType<typeof setInterval> | null = null;

  const running = $derived(status?.containers.filter((c) => c.state === "running").length ?? 0);

  const groups = $derived.by(() => {
    const cs = status?.containers ?? [];
    const fg = folders.map((f) => ({
      id: f.id,
      name: f.name,
      items: cs.filter((c) => assign[c.name] === f.id),
    }));
    const ungrouped = cs.filter((c) => !assign[c.name] || !folders.some((f) => f.id === assign[c.name]));
    return { folders: fg, ungrouped };
  });

  function persist() {
    try {
      localStorage.setItem(FK, JSON.stringify(folders));
      localStorage.setItem(AK, JSON.stringify(assign));
    } catch {
      /* localStorage indispo : on ignore */
    }
  }

  function loadFolders() {
    try {
      folders = JSON.parse(localStorage.getItem(FK) || "[]");
      assign = JSON.parse(localStorage.getItem(AK) || "{}");
    } catch {
      folders = [];
      assign = {};
    }
  }

  function addFolder() {
    const name = newFolder.trim();
    if (!name) return;
    const id = crypto.randomUUID?.() ?? `f${folders.length}-${name}`;
    folders = [...folders, { id, name }];
    newFolder = "";
    creating = false;
    persist();
  }

  function removeFolder(id: string) {
    folders = folders.filter((f) => f.id !== id);
    for (const k of Object.keys(assign)) if (assign[k] === id) delete assign[k];
    assign = { ...assign };
    persist();
  }

  function moveTo(name: string, folderId: string) {
    if (folderId) assign[name] = folderId;
    else delete assign[name];
    assign = { ...assign };
    persist();
  }

  async function load() {
    try {
      status = await dockerList(profileId, password);
      error = null;
      if (selected) selected = status.containers.find((c) => c.id === selected!.id) ?? null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function act(action: "start" | "stop" | "restart") {
    if (!selected) return;
    acting = true;
    error = null;
    try {
      await dockerAction(profileId, selected.id, action, password);
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      acting = false;
    }
  }

  function open(c: Container) {
    selected = c;
    logsOpen = false;
    logsText = "";
  }
  function close() {
    selected = null;
  }

  async function toggleLogs() {
    if (!selected) return;
    logsOpen = !logsOpen;
    if (logsOpen && !logsText) {
      logsLoading = true;
      try {
        logsText = (await dockerLogs(profileId, selected.id, 200, password)) || "(aucune sortie)";
      } catch (e) {
        logsText = String(e);
      } finally {
        logsLoading = false;
      }
    }
  }

  onMount(() => {
    loadFolders();
    load();
    timer = setInterval(load, 10000);
  });
  onDestroy(() => {
    if (timer) clearInterval(timer);
  });
</script>

<section class="panel" in:fly={{ y: 14, duration: 360, easing: quintOut }}>
  <div class="head">
    <h2>
      Conteneurs
      {#if status?.installed && status.containers.length}<span class="count">{running}/{status.containers.length} actifs</span>{/if}
    </h2>
    <div class="head-actions">
      {#if status?.installed && status.containers.length}
        <button class="chip" onclick={() => (creating = !creating)}><Icon name="folder" size={15} /> Dossier</button>
      {/if}
      <button class="icon-btn" title="Rafraîchir" onclick={load} disabled={loading}>
        <Icon name="refresh" size={17} spin={loading && !status} />
      </button>
    </div>
  </div>

  {#if creating}
    <form class="new-folder" transition:slide={{ duration: 200 }} onsubmit={(e) => { e.preventDefault(); addFolder(); }}>
      <input bind:value={newFolder} placeholder="Nom du dossier…" autocomplete="off" />
      <button class="chip solid" type="submit">Créer</button>
    </form>
  {/if}

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
    {#each groups.folders as f (f.id)}
      <div class="folder" transition:slide={{ duration: 200 }}>
        <div class="folder-head">
          <span><Icon name="folder" size={15} /> {f.name} <span class="count">{f.items.length}</span></span>
          <button class="mini" title="Supprimer le dossier" onclick={() => removeFolder(f.id)}><Icon name="close" size={14} /></button>
        </div>
        {#if f.items.length}
          <div class="tiles">
            {#each f.items as c (c.id)}
              {@render tile(c)}
            {/each}
          </div>
        {:else}
          <p class="folder-empty">Vide — déplace un conteneur ici depuis son détail.</p>
        {/if}
      </div>
    {/each}

    {#if groups.ungrouped.length}
      <div class="tiles top">
        {#each groups.ungrouped as c (c.id)}
          {@render tile(c)}
        {/each}
      </div>
    {/if}
  {/if}
</section>

{#snippet tile(c: Container)}
  <button class="tile" onclick={() => open(c)} in:fly={{ y: 10, duration: 240, easing: quintOut }}>
    <span class="tile-icon" class:on={c.state === "running"}>
      <Icon name="server" size={26} />
      <span class="dot" class:running={c.state === "running"}></span>
    </span>
    <span class="tile-name">{c.name}</span>
  </button>
{/snippet}

{#if selected}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="overlay"
    role="presentation"
    transition:fade={{ duration: 150 }}
    onclick={(e) => { if (e.target === e.currentTarget) close(); }}
  >
    <div class="modal" role="dialog" aria-modal="true" transition:scale={{ duration: 200, start: 0.95, easing: quintOut }}>
      <div class="modal-head">
        <span class="tile-icon lg" class:on={selected.state === "running"}><Icon name="server" size={24} /></span>
        <div class="modal-title">
          <strong>{selected.name}</strong>
          <span>{selected.image}</span>
        </div>
        <button class="icon-btn" onclick={close}><Icon name="close" size={18} /></button>
      </div>

      <div class="modal-meta">
        <span class="badge" class:live={selected.state === "running"}>{selected.state === "running" ? "En marche" : "Arrêté"}</span>
        <span class="muted">{selected.status}</span>
        {#if selected.state === "running" && selected.cpuPercent != null}
          <span class="muted">· CPU {selected.cpuPercent.toFixed(0)}%{#if selected.memUsage} · {selected.memUsage.split(" / ")[0]}{/if}</span>
        {/if}
      </div>

      <div class="modal-actions">
        {#if selected.state === "running"}
          <button class="act" onclick={() => act("restart")} disabled={acting}><Icon name={acting ? "spinner" : "refresh"} size={16} spin={acting} /> Redémarrer</button>
          <button class="act" onclick={() => act("stop")} disabled={acting}><Icon name="stop" size={16} /> Arrêter</button>
        {:else}
          <button class="act start" onclick={() => act("start")} disabled={acting}><Icon name={acting ? "spinner" : "play"} size={16} spin={acting} /> Démarrer</button>
        {/if}
        <button class="act" class:active={logsOpen} onclick={toggleLogs}><Icon name="logs" size={16} /> Logs</button>
      </div>

      {#if folders.length}
        <label class="folder-select">
          <span>Dossier</span>
          <select value={assign[selected.name] ?? ""} onchange={(e) => moveTo(selected!.name, e.currentTarget.value)}>
            <option value="">Sans dossier</option>
            {#each folders as f (f.id)}<option value={f.id}>{f.name}</option>{/each}
          </select>
        </label>
      {/if}

      {#if logsOpen}
        <div class="logs" transition:slide={{ duration: 200 }}>
          {#if logsLoading}<span class="muted"><Icon name="spinner" size={13} spin /> Chargement…</span>{:else}<pre>{logsText}</pre>{/if}
        </div>
      {/if}

      {#if error}<div class="state err small"><Icon name="alert" size={14} /> {error}</div>{/if}
    </div>
  </div>
{/if}

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
  .head-actions {
    display: flex;
    gap: 0.4rem;
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
  }
  .count {
    font-size: 0.76rem;
    color: rgba(255, 255, 255, 0.45);
    font-weight: 400;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.4rem 0.7rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.05);
    color: #e7ecf3;
    font-size: 0.82rem;
    cursor: pointer;
  }
  .chip:hover {
    background: rgba(255, 255, 255, 0.12);
  }
  .chip.solid {
    background: #fff;
    color: #000;
    border-color: transparent;
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
  }
  .icon-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }

  .new-folder {
    display: flex;
    gap: 0.4rem;
    margin-bottom: 1rem;
  }
  .new-folder input {
    flex: 1;
    padding: 0.55rem 0.75rem;
    border-radius: 9px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    background: rgba(0, 0, 0, 0.35);
    color: #f4f4f5;
    font-size: 0.86rem;
    outline: none;
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
    color: rgba(255, 255, 255, 0.42);
    font-size: 0.8rem;
  }

  .folder {
    margin-bottom: 1rem;
  }
  .folder-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.85rem;
    margin-bottom: 0.6rem;
  }
  .folder-head > span {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }
  .mini {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.6);
    cursor: pointer;
  }
  .mini:hover {
    background: rgba(255, 90, 90, 0.18);
    color: #fca5a5;
  }
  .folder-empty {
    margin: 0;
    font-size: 0.78rem;
    color: rgba(255, 255, 255, 0.35);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(88px, 1fr));
    gap: 1rem;
  }
  .tiles.top {
    margin-top: 0.2rem;
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.45rem;
    border: none;
    background: transparent;
    cursor: pointer;
    padding: 0;
    color: #cdd6e6;
  }
  .tile-icon {
    position: relative;
    display: grid;
    place-items: center;
    width: 60px;
    height: 60px;
    border-radius: 17px;
    color: #fff;
    background: linear-gradient(155deg, #4b5563, #1f2937);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
    transition: transform 0.22s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .tile-icon.on {
    background: linear-gradient(155deg, #2ee6c7, #0d9488);
  }
  .tile-icon.lg {
    width: 46px;
    height: 46px;
    border-radius: 13px;
  }
  .tile:hover .tile-icon {
    transform: translateY(-3px) scale(1.05);
  }
  .tile:active .tile-icon {
    transform: scale(0.98);
  }
  .dot {
    position: absolute;
    top: -3px;
    right: -3px;
    width: 13px;
    height: 13px;
    border-radius: 50%;
    background: #6b7280;
    border: 2.5px solid #17171a;
  }
  .dot.running {
    background: #4ade80;
  }
  .tile-name {
    font-size: 0.78rem;
    color: rgba(255, 255, 255, 0.75);
    max-width: 84px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 1.5rem;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }
  .modal {
    width: 100%;
    max-width: 460px;
    padding: 1.3rem;
    border-radius: 18px;
    background: rgba(24, 24, 27, 0.92);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 30px 70px rgba(0, 0, 0, 0.6);
  }
  .modal-head {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    margin-bottom: 1rem;
  }
  .modal-title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .modal-title strong {
    font-size: 1.05rem;
  }
  .modal-title span {
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .modal-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }
  .badge {
    font-size: 0.74rem;
    font-weight: 600;
    padding: 0.15rem 0.5rem;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.7);
  }
  .badge.live {
    background: rgba(74, 222, 128, 0.16);
    color: #4ade80;
  }
  .modal-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .act {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.55rem 0.9rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.06);
    color: #e7ecf3;
    font-size: 0.86rem;
    cursor: pointer;
    transition: background 0.15s, transform 0.12s;
  }
  .act:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.13);
  }
  .act:active:not(:disabled) {
    transform: scale(0.96);
  }
  .act:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .act.start {
    color: #4ade80;
    border-color: rgba(74, 222, 128, 0.4);
  }
  .act.active {
    background: rgba(255, 255, 255, 0.14);
  }

  .folder-select {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    margin-top: 1rem;
    font-size: 0.84rem;
    color: rgba(255, 255, 255, 0.6);
  }
  .folder-select select {
    padding: 0.45rem 0.7rem;
    border-radius: 9px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    background: rgba(0, 0, 0, 0.4);
    color: #f4f4f5;
    font-size: 0.84rem;
    outline: none;
  }

  .logs {
    margin-top: 0.9rem;
  }
  .logs pre {
    margin: 0;
    max-height: 240px;
    overflow: auto;
    padding: 0.7rem 0.8rem;
    border-radius: 10px;
    background: rgba(0, 0, 0, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.08);
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 0.74rem;
    color: rgba(255, 255, 255, 0.8);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
