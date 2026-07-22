<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "$lib/Icon.svelte";
  import { composeUp, deployApp, dockerList, installDocker } from "$lib/api";
  import { addApp } from "$lib/installed";
  import { CATALOG, genPassword, type AppTemplate, type DeployConfig } from "$lib/catalog";

  let {
    profileId,
    password,
    onClose,
    onDeployed,
  }: { profileId: string; password?: string; onClose: () => void; onDeployed: () => void } = $props();

  let installed = $state<boolean | null>(null);
  let installing = $state(false);
  let installErr = $state<string | null>(null);

  let selected = $state<AppTemplate | null>(null);
  let cfgName = $state("");
  let cfgPorts = $state<number[]>([]);
  let cfgEnv = $state<{ key: string; label: string; value: string; generated: boolean }[]>([]);

  let deploying = $state(false);
  let deployErr = $state<string | null>(null);
  let deployOk = $state<string | null>(null);

  let query = $state("");
  let failed = $state<Record<string, boolean>>({});

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return CATALOG;
    return CATALOG.filter(
      (a) =>
        a.name.toLowerCase().includes(q) ||
        a.category.toLowerCase().includes(q) ||
        a.description.toLowerCase().includes(q) ||
        a.image.toLowerCase().includes(q),
    );
  });

  async function checkDocker() {
    try {
      const s = await dockerList(profileId, password);
      installed = s.installed;
    } catch {
      installed = false;
    }
  }

  async function installDockerNow() {
    installing = true;
    installErr = null;
    try {
      await installDocker(profileId, password);
      installed = true;
      onDeployed();
    } catch (e) {
      installErr = String(e);
    } finally {
      installing = false;
    }
  }

  function selectApp(t: AppTemplate) {
    selected = t;
    deployErr = null;
    deployOk = null;
    cfgName = t.id;
    cfgPorts = t.ports.map((p) => p.defaultHost);
    cfgEnv = t.env.map((e) => ({
      key: e.key,
      label: e.label ?? e.key,
      value: e.generate ? genPassword() : (e.value ?? ""),
      generated: !!e.generate,
    }));
  }

  async function install() {
    if (!selected) return;
    const name = cfgName.trim();
    deploying = true;
    deployErr = null;
    try {
      if (selected.compose) {
        // App multi-conteneurs : substitue les placeholders et lance docker compose.
        let yaml = selected.compose
          .replace(/\{\{NAME\}\}/g, name)
          .replace(/\{\{PORT\}\}/g, String(cfgPorts[0]));
        for (const e of cfgEnv) yaml = yaml.split(`{{${e.key}}}`).join(e.value);
        await composeUp(profileId, name, yaml, password);
        deployOk = `Stack « ${name} » déployée (${cfgPorts[0] ? `port ${cfgPorts[0]}` : "en cours"}).`;
      } else {
        const config: DeployConfig = {
          name,
          image: selected.image,
          ports: selected.ports.map((p, i) => ({ host: cfgPorts[i], container: p.container })),
          env: cfgEnv.map((e) => ({ key: e.key, value: e.value })),
          volumes: selected.volumes.map((v) => v.replace(/\{name\}/g, name)),
        };
        await deployApp(profileId, config, password);
        deployOk = `Conteneur « ${name} » déployé.`;
      }
      addApp(profileId, {
        id: crypto.randomUUID?.() ?? `a-${name}`,
        catalogId: selected.id,
        label: selected.name,
        name,
        port: cfgPorts[0] ?? null,
        stack: !!selected.compose,
      });
      onDeployed();
    } catch (e) {
      deployErr = String(e);
    } finally {
      deploying = false;
    }
  }

  function copy(v: string) {
    navigator.clipboard?.writeText(v);
  }

  onMount(checkDocker);
</script>

{#snippet appIcon(app: AppTemplate, size: number)}
  {#if failed[app.id]}
    <span class="mono" style="background:{app.color};width:{size}px;height:{size}px">{app.mono}</span>
  {:else}
    <span class="icon-tile" style="width:{size}px;height:{size}px">
      <img
        src="/icons/{app.id}.svg"
        alt={app.name}
        onerror={() => (failed = { ...failed, [app.id]: true })}
      />
    </span>
  {/if}
{/snippet}

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="overlay"
  role="presentation"
  transition:fade={{ duration: 150 }}
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
>
  <div class="store" role="dialog" aria-modal="true" transition:scale={{ duration: 200, start: 0.96, easing: quintOut }}>
    <div class="store-head">
      {#if selected}
        <button class="icon-btn back" onclick={() => (selected = null)}><Icon name="arrow" size={18} /></button>
      {/if}
      <h2>{selected ? selected.name : "App Store"}</h2>
      <button class="icon-btn" onclick={onClose}><Icon name="close" size={18} /></button>
    </div>

    {#if installed === null}
      <div class="state"><Icon name="spinner" size={22} spin /> Vérification de Docker…</div>
    {:else if !installed}
      <div class="install">
        <div class="install-icon"><Icon name="server" size={30} /></div>
        <p class="install-title">Docker est requis pour l'App Store</p>
        <p class="muted">
          Beacon peut l'installer sur ton serveur via le script officiel (get.docker.com). Ça peut
          prendre une minute.
        </p>
        {#if installErr}<div class="err">{installErr}</div>{/if}
        <button class="primary" onclick={installDockerNow} disabled={installing}>
          {#if installing}<Icon name="spinner" size={16} spin /> Installation…{:else}Installer Docker{/if}
        </button>
      </div>
    {:else if selected}
      <div class="config" in:fly={{ y: 10, duration: 220, easing: quintOut }}>
        <div class="cfg-head">
          {@render appIcon(selected, 46)}
          <div>
            <strong>{selected.name}</strong>
            <span class="muted">{selected.image}</span>
          </div>
        </div>

        {#if deployOk}
          <div class="ok">
            <Icon name="check" size={16} />
            <div>
              <strong>{deployOk}</strong>
              {#if cfgPorts[0]}<span class="muted">Accessible sur le port {cfgPorts[0]}.</span>{/if}
            </div>
          </div>
        {:else}
          <label>
            <span>Nom du conteneur</span>
            <input bind:value={cfgName} autocomplete="off" />
          </label>

          {#each selected.ports as p, i (p.container)}
            <label>
              <span>Port hôte <em>(→ {p.container} dans le conteneur)</em></span>
              <input type="number" bind:value={cfgPorts[i]} min="1" max="65535" />
            </label>
          {/each}

          {#each cfgEnv as e (e.key)}
            <label>
              <span>{e.label}</span>
              <div class="secret-row">
                <input bind:value={e.value} readonly={e.generated} autocomplete="off" />
                {#if e.generated}
                  <button type="button" class="mini" title="Copier" onclick={() => copy(e.value)}><Icon name="logs" size={15} /></button>
                {/if}
              </div>
            </label>
          {/each}

          {#if deployErr}<div class="err">{deployErr}</div>{/if}

          <button class="primary" onclick={install} disabled={deploying || cfgName.trim() === ""}>
            {#if deploying}<Icon name="spinner" size={16} spin /> Déploiement… (pull de l'image){:else}<Icon name="plus" size={16} /> Installer{/if}
          </button>
          <p class="hint">L'image sera téléchargée depuis Docker Hub sur ton serveur.</p>
        {/if}
      </div>
    {:else}
      <div class="search">
        <Icon name="search" size={18} />
        <input bind:value={query} placeholder="Rechercher une application…" autocomplete="off" />
        {#if query}<button class="clear" onclick={() => (query = "")}><Icon name="close" size={15} /></button>{/if}
      </div>

      {#if filtered.length === 0}
        <div class="state">Aucune application ne correspond à « {query} ».</div>
      {:else}
        <div class="grid">
          {#each filtered as app, i (app.id)}
            <button class="card" onclick={() => selectApp(app)} in:fly={{ y: 12, duration: 240, delay: Math.min(i, 10) * 25, easing: quintOut }}>
              {@render appIcon(app, 46)}
              <div class="card-body">
                <strong>{app.name}</strong>
                <span class="cat">{app.category}</span>
                <span class="desc">{app.description}</span>
              </div>
            </button>
          {/each}
        </div>
      {/if}
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
  .store {
    width: 100%;
    max-width: 760px;
    max-height: 86vh;
    overflow-y: auto;
    padding: 1.4rem;
    border-radius: 20px;
    background: rgba(24, 24, 27, 0.94);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.65);
  }
  .store-head {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    margin-bottom: 1.2rem;
  }
  .store-head h2 {
    margin: 0;
    font-size: 1.2rem;
    flex: 1;
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
  .icon-btn.back :global(svg) {
    transform: rotate(180deg);
  }

  .muted {
    color: rgba(255, 255, 255, 0.45);
    font-size: 0.82rem;
  }
  .state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    padding: 3rem;
    color: rgba(255, 255, 255, 0.6);
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
    max-width: 420px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem 1rem;
    border-radius: 12px;
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.5);
    margin-bottom: 1rem;
  }
  .search input {
    flex: 1;
    border: none;
    background: transparent;
    color: #f4f4f5;
    font-size: 0.92rem;
    outline: none;
  }
  .clear {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: none;
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.08);
    color: #cdd6e6;
    cursor: pointer;
  }
  .clear:hover {
    background: rgba(255, 255, 255, 0.16);
  }

  .icon-tile {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    border-radius: 13px;
    background: #fff;
    padding: 9px;
    box-shadow: 0 8px 18px rgba(0, 0, 0, 0.4);
  }
  .icon-tile img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.8rem;
  }
  .card {
    display: flex;
    gap: 0.8rem;
    text-align: left;
    padding: 0.9rem;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.07);
    color: #e7ecf3;
    cursor: pointer;
    transition: transform 0.18s cubic-bezier(0.22, 1, 0.36, 1), background 0.18s;
  }
  .card:hover {
    transform: translateY(-2px);
    background: rgba(255, 255, 255, 0.08);
  }
  .card-body {
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
    min-width: 0;
  }
  .card-body strong {
    font-size: 0.95rem;
  }
  .cat {
    font-size: 0.7rem;
    color: rgba(255, 255, 255, 0.4);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .desc {
    font-size: 0.78rem;
    color: rgba(255, 255, 255, 0.55);
  }

  .mono {
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    flex-shrink: 0;
    border-radius: 13px;
    color: #fff;
    font-weight: 700;
    font-size: 1.05rem;
    box-shadow: 0 8px 18px rgba(0, 0, 0, 0.4);
  }

  .config {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }
  .cfg-head {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .cfg-head div {
    display: flex;
    flex-direction: column;
  }
  .cfg-head strong {
    font-size: 1.05rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.82rem;
    color: rgba(255, 255, 255, 0.6);
  }
  label em {
    color: rgba(255, 255, 255, 0.35);
    font-style: normal;
  }
  input {
    padding: 0.6rem 0.8rem;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(0, 0, 0, 0.4);
    color: #f4f4f5;
    font-size: 0.9rem;
    outline: none;
  }
  input:focus {
    border-color: rgba(255, 255, 255, 0.4);
  }
  .secret-row {
    display: flex;
    gap: 0.4rem;
  }
  .secret-row input {
    flex: 1;
    font-family: ui-monospace, monospace;
    font-size: 0.82rem;
  }
  .mini {
    display: grid;
    place-items: center;
    width: 38px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.05);
    color: #cdd6e6;
    cursor: pointer;
  }
  .mini:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    padding: 0.75rem;
    border: none;
    border-radius: 12px;
    background: #fff;
    color: #000;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    transition: filter 0.15s, opacity 0.15s;
  }
  .primary:hover:not(:disabled) {
    filter: brightness(0.9);
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .hint {
    margin: 0;
    text-align: center;
    font-size: 0.76rem;
    color: rgba(255, 255, 255, 0.4);
  }

  .err {
    padding: 0.7rem 0.9rem;
    border-radius: 11px;
    background: rgba(255, 90, 90, 0.12);
    border: 1px solid rgba(255, 120, 120, 0.32);
    color: #fecaca;
    font-size: 0.84rem;
    word-break: break-word;
  }
  .ok {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.9rem 1rem;
    border-radius: 12px;
    background: rgba(74, 222, 128, 0.14);
    border: 1px solid rgba(74, 222, 128, 0.3);
    color: #bbf7d0;
  }
  .ok div {
    display: flex;
    flex-direction: column;
  }
</style>
