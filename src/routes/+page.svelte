<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "$lib/Icon.svelte";
  import Dashboard from "$lib/Dashboard.svelte";
  import {
    deleteProfile,
    hardenBootstrap,
    listProfiles,
    pickKeyFile,
    saveProfile,
    sshTestConnection,
  } from "$lib/api";
  import type {
    AuthInput,
    ExecOutcome,
    HardeningReport,
    ProfileMeta,
    SaveAuth,
  } from "$lib/types";

  let view = $state<"list" | "form" | "dashboard">("list");
  let profiles = $state<ProfileMeta[]>([]);

  // Dashboard (serveur actif)
  let activeProfile = $state<ProfileMeta | null>(null);
  let activePassword = $state<string | undefined>(undefined);

  // Formulaire d'ajout
  let label = $state("");
  let host = $state("");
  let port = $state(22);
  let username = $state("root");
  let authKind = $state<"key" | "password">("key");
  let keyPath = $state("");
  let passphrase = $state("");
  let password = $state("");

  let testing = $state(false);
  let testedFp = $state<string | null>(null);
  let testOutput = $state<string | null>(null);
  let formError = $state<string | null>(null);
  let saving = $state(false);

  // Durcissement (root uniquement)
  let devUsername = $state("dev");
  let hardening = $state(false);
  let hardenReport = $state<HardeningReport | null>(null);

  // Connexion depuis la liste
  let passwordPromptId = $state<string | null>(null);
  let promptPassword = $state("");

  const canTest = $derived(
    host.trim() !== "" &&
      username.trim() !== "" &&
      (authKind === "key" ? keyPath.trim() !== "" : password !== ""),
  );

  onMount(refresh);

  async function refresh() {
    try {
      profiles = await listProfiles();
    } catch (e) {
      formError = String(e);
    }
  }

  function openForm() {
    label = "";
    host = "";
    port = 22;
    username = "root";
    authKind = "key";
    keyPath = "";
    passphrase = "";
    password = "";
    testing = false;
    testedFp = null;
    testOutput = null;
    formError = null;
    devUsername = "dev";
    hardenReport = null;
    view = "form";
  }

  async function browseKey() {
    const picked = await pickKeyFile();
    if (picked) keyPath = picked;
  }

  function buildAuth(): AuthInput {
    return authKind === "key"
      ? { kind: "key", path: keyPath.trim(), passphrase: passphrase || null }
      : { kind: "password", password };
  }

  async function testConn(event: Event) {
    event.preventDefault();
    testing = true;
    testOutput = null;
    formError = null;
    testedFp = null;
    try {
      const outcome: ExecOutcome = await sshTestConnection({
        host: host.trim(),
        port,
        username: username.trim(),
        auth: buildAuth(),
      });
      testOutput = outcome.result.stdout.trim();
      testedFp = outcome.host_key_fp;
    } catch (e) {
      formError = String(e);
    } finally {
      testing = false;
    }
  }

  async function save() {
    saving = true;
    formError = null;
    try {
      const auth: SaveAuth =
        authKind === "key"
          ? { kind: "key", path: keyPath.trim(), passphrase: passphrase || null }
          : { kind: "password" };
      await saveProfile({
        label: label.trim() || host.trim(),
        host: host.trim(),
        port,
        username: username.trim(),
        auth,
        hostKeyFp: testedFp,
      });
      await refresh();
      view = "list";
    } catch (e) {
      formError = String(e);
    } finally {
      saving = false;
    }
  }

  async function hardenServer() {
    hardening = true;
    hardenReport = null;
    formError = null;
    try {
      const report = await hardenBootstrap({
        host: host.trim(),
        port,
        rootUsername: username.trim(),
        auth: buildAuth(),
        devUsername: devUsername.trim(),
        label: label.trim(),
      });
      hardenReport = report;
      if (report.success) await refresh();
    } catch (e) {
      formError = String(e);
    } finally {
      hardening = false;
    }
  }

  function stepIcon(status: string): string {
    if (status === "failed") return "alert";
    if (status === "skipped") return "arrow";
    return "check";
  }

  function openDashboard(profile: ProfileMeta, pwd?: string) {
    activeProfile = profile;
    activePassword = pwd;
    passwordPromptId = null;
    promptPassword = "";
    view = "dashboard";
  }

  function onConnectClick(profile: ProfileMeta) {
    if (profile.authKind === "password") {
      passwordPromptId = profile.id;
      promptPassword = "";
    } else {
      openDashboard(profile);
    }
  }

  function backToList() {
    view = "list";
    activeProfile = null;
    activePassword = undefined;
    refresh();
  }

  async function remove(profile: ProfileMeta) {
    try {
      await deleteProfile(profile.id);
      await refresh();
    } catch (e) {
      formError = String(e);
    }
  }
</script>

<main>
  <div class="shell">
    <header>
      <span class="brand-icon"><Icon name="beacon" size={30} /></span>
      <div>
        <h1>Beacon</h1>
        <p class="tagline">Pilote ton serveur — VPS ou machine locale.</p>
      </div>
    </header>

    {#if view === "dashboard" && activeProfile}
      <Dashboard profile={activeProfile} password={activePassword} onBack={backToList} />
    {:else if view === "list"}
      <section class="panel">
        <div class="panel-head">
          <h2>Mes serveurs</h2>
          <button class="btn primary" onclick={openForm}>
            <Icon name="plus" size={18} /> Ajouter un serveur
          </button>
        </div>

        {#if profiles.length === 0}
          <div class="empty">
            <Icon name="server" size={40} />
            <p>Aucun serveur enregistré.</p>
            <button class="btn primary" onclick={openForm}>
              <Icon name="plus" size={18} /> Ajouter ton premier serveur
            </button>
          </div>
        {:else}
          <ul class="cards">
            {#each profiles as p (p.id)}
              <li class="card">
                <div class="card-main">
                  <span class="card-icon"><Icon name="server" size={22} /></span>
                  <div class="card-info">
                    <strong>{p.label}</strong>
                    <span class="sub">{p.username}@{p.host}:{p.port}</span>
                    <span class="badge">
                      <Icon name={p.authKind === "key" ? "key" : "lock"} size={13} />
                      {p.authKind === "key" ? "Clé SSH" : "Mot de passe"}
                      {#if p.hostKeyFp}
                        <span class="pinned"><Icon name="lock" size={12} /> hôte épinglé</span>
                      {/if}
                    </span>
                  </div>
                </div>

                <div class="card-actions">
                  <button class="btn primary sm" onclick={() => onConnectClick(p)}>
                    <Icon name="arrow" size={16} /> Ouvrir
                  </button>
                  <button class="btn ghost sm" title="Supprimer" onclick={() => remove(p)}>
                    <Icon name="trash" size={16} />
                  </button>
                </div>

                {#if passwordPromptId === p.id}
                  <form
                    class="pwd-prompt"
                    onsubmit={(e) => {
                      e.preventDefault();
                      openDashboard(p, promptPassword);
                    }}
                  >
                    <input
                      type="password"
                      placeholder="Mot de passe (non enregistré)"
                      bind:value={promptPassword}
                    />
                    <button class="btn primary sm" type="submit">Ouvrir</button>
                  </form>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {:else}
      <section class="panel">
        <div class="panel-head">
          <h2>Ajouter un serveur</h2>
          <button class="btn ghost" onclick={() => (view = "list")}>
            <Icon name="close" size={18} /> Annuler
          </button>
        </div>

        <form class="add-form" onsubmit={testConn}>
          <label class="span-2">
            <span>Nom <em>(optionnel)</em></span>
            <input bind:value={label} placeholder="Mon VPS de prod" autocomplete="off" />
          </label>

          <div class="grid">
            <label class="span-2">
              <span>Adresse IP / hôte</span>
              <input bind:value={host} placeholder="192.168.1.50 ou 203.0.113.10" autocomplete="off" />
            </label>
            <label>
              <span>Port</span>
              <input type="number" bind:value={port} min="1" max="65535" />
            </label>
            <label>
              <span>Utilisateur</span>
              <input bind:value={username} placeholder="root" autocomplete="off" />
            </label>
          </div>

          <div class="auth-toggle">
            <button type="button" class:active={authKind === "key"} onclick={() => (authKind = "key")}>
              <Icon name="key" size={15} /> Clé SSH
            </button>
            <button
              type="button"
              class:active={authKind === "password"}
              onclick={() => (authKind = "password")}
            >
              <Icon name="lock" size={15} /> Mot de passe
            </button>
          </div>

          {#if authKind === "key"}
            <label>
              <span>Clé privée</span>
              <div class="file-row">
                <input bind:value={keyPath} placeholder="Choisis ta clé SSH…" readonly />
                <button type="button" class="btn ghost sm" onclick={browseKey}>
                  <Icon name="folder" size={16} /> Parcourir
                </button>
              </div>
            </label>
            <label>
              <span>Passphrase <em>(si la clé en a une)</em></span>
              <input type="password" bind:value={passphrase} autocomplete="off" />
            </label>
          {:else}
            <label>
              <span>Mot de passe <em>(bootstrap, jamais enregistré)</em></span>
              <input type="password" bind:value={password} autocomplete="off" />
            </label>
          {/if}

          <div class="actions">
            <button class="btn" type="submit" disabled={!canTest || testing}>
              {#if testing}
                <Icon name="spinner" size={16} spin /> Test…
              {:else}
                Tester la connexion
              {/if}
            </button>
            <button class="btn primary" type="button" onclick={save} disabled={!canTest || saving}>
              {#if saving}
                <Icon name="spinner" size={16} spin /> Enregistrement…
              {:else}
                <Icon name="check" size={16} /> Enregistrer
              {/if}
            </button>
          </div>
        </form>

        {#if testOutput}
          <div class="feedback ok">
            <span class="fb-title"><Icon name="check" size={16} /> Connexion réussie</span>
            <code>{testOutput}</code>
            {#if testedFp}
              <span class="fp">Clé d'hôte : {testedFp}</span>
            {/if}
          </div>
        {/if}
        {#if formError}
          <div class="feedback err">
            <span class="fb-title"><Icon name="alert" size={16} /> Échec</span>
            <span>{formError}</span>
          </div>
        {/if}

        {#if username.trim() === "root"}
          <div class="harden-box">
            <div class="harden-head">
              <Icon name="lock" size={18} /> Sécuriser ce serveur <span class="tag">recommandé</span>
            </div>
            <p>
              Beacon va créer un utilisateur dédié, générer une clé SSH et désactiver le login
              root et par mot de passe. Rien n'est désactivé tant que la nouvelle clé n'est pas
              vérifiée — ton accès actuel reste intact en cas de souci.
            </p>
            <label>
              <span>Nom de l'utilisateur à créer</span>
              <input bind:value={devUsername} placeholder="dev" autocomplete="off" />
            </label>
            <button
              class="btn primary"
              type="button"
              onclick={hardenServer}
              disabled={!canTest || hardening}
            >
              {#if hardening}
                <Icon name="spinner" size={16} spin /> Sécurisation…
              {:else}
                <Icon name="lock" size={16} /> Sécuriser le serveur
              {/if}
            </button>
          </div>
        {/if}

        {#if hardenReport}
          <div class="steps">
            {#each hardenReport.steps as s (s.key)}
              <div class="step {s.status}">
                <Icon name={stepIcon(s.status)} size={16} />
                <div>
                  <span>{s.label}</span>
                  {#if s.detail}<em>{s.detail}</em>{/if}
                </div>
              </div>
            {/each}
            <div class="harden-msg {hardenReport.success ? 'ok' : 'err'}">
              <Icon name={hardenReport.success ? "check" : "alert"} size={16} />
              {hardenReport.message}
            </div>
          </div>
        {/if}
      </section>
    {/if}

    <p class="privacy">
      <Icon name="lock" size={13} /> 100 % local — aucune donnée ne quitte ta machine, hors la connexion à ton serveur.
    </p>
  </div>
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    overflow-x: hidden;
  }
  main {
    min-height: 100vh;
    display: grid;
    place-items: start center;
    padding: 2.5rem 1rem 3rem;
    font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
    color: #e7ecf3;
    background: radial-gradient(1200px 700px at 50% -12%, #1b2a4a 0%, #0c1220 55%, #080b14 100%);
  }
  .shell {
    width: 100%;
    max-width: 620px;
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    margin-bottom: 1.6rem;
  }
  .brand-icon {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    border-radius: 15px;
    background: linear-gradient(180deg, #3b82f6, #2563eb);
    color: white;
    box-shadow: 0 8px 24px rgba(37, 99, 235, 0.4);
  }
  h1 {
    margin: 0;
    font-size: 1.55rem;
    letter-spacing: -0.02em;
  }
  .tagline {
    margin: 0.15rem 0 0;
    color: #93a1bd;
    font-size: 0.85rem;
  }

  .panel {
    background: rgba(20, 27, 44, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 18px;
    padding: 1.4rem;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(12px);
  }
  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.1rem;
  }
  h2 {
    margin: 0;
    font-size: 1.05rem;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.7rem;
    padding: 2.4rem 1rem;
    color: #8695b3;
    text-align: center;
  }
  .empty p {
    margin: 0;
  }

  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .card {
    background: rgba(9, 13, 24, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 14px;
    padding: 0.9rem 1rem;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.7rem;
  }
  .card-main {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    min-width: 0;
  }
  .card-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 11px;
    background: rgba(59, 130, 246, 0.16);
    color: #7ab0ff;
  }
  .card-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
  }
  .card-info strong {
    font-size: 0.98rem;
  }
  .sub {
    color: #93a1bd;
    font-size: 0.82rem;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    color: #7d88a3;
    font-size: 0.74rem;
    margin-top: 0.1rem;
  }
  .pinned {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    color: #62c082;
    margin-left: 0.3rem;
  }
  .card-actions {
    display: flex;
    gap: 0.4rem;
  }
  .pwd-prompt,
  .feedback {
    flex-basis: 100%;
  }
  .pwd-prompt {
    display: flex;
    gap: 0.4rem;
    margin-top: 0.3rem;
  }
  .pwd-prompt input {
    flex: 1;
  }

  .add-form {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.85rem;
  }
  .span-2 {
    grid-column: 1 / -1;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.82rem;
    color: #aeb9d1;
  }
  label em {
    color: #7d88a3;
    font-style: normal;
  }
  input {
    padding: 0.65rem 0.8rem;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(9, 13, 24, 0.65);
    color: #f2f5fa;
    font-size: 0.92rem;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.25);
  }
  .file-row {
    display: flex;
    gap: 0.4rem;
  }
  .file-row input {
    flex: 1;
  }

  .auth-toggle {
    display: flex;
    gap: 0.4rem;
    background: rgba(9, 13, 24, 0.6);
    padding: 0.3rem;
    border-radius: 12px;
  }
  .auth-toggle button {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.55rem;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: #aeb9d1;
    font-size: 0.86rem;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .auth-toggle button.active {
    background: #2563eb;
    color: white;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.3rem;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.65rem 1rem;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.05);
    color: #e7ecf3;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: filter 0.15s, background 0.15s, opacity 0.15s;
  }
  .btn.sm {
    padding: 0.5rem 0.7rem;
    font-size: 0.84rem;
  }
  .btn.primary {
    border: none;
    background: linear-gradient(180deg, #3b82f6, #2563eb);
    color: white;
  }
  .btn.ghost {
    background: transparent;
  }
  .btn:hover:not(:disabled) {
    filter: brightness(1.1);
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .actions .btn {
    flex: 1;
  }

  .feedback {
    margin-top: 1rem;
    padding: 0.85rem 1rem;
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.86rem;
  }
  .feedback.ok {
    background: rgba(22, 101, 52, 0.22);
    border: 1px solid rgba(74, 222, 128, 0.32);
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
  .feedback code {
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 0.78rem;
    color: #c7f9cc;
    word-break: break-word;
  }
  .fp {
    font-family: ui-monospace, monospace;
    font-size: 0.72rem;
    color: #8fd3a6;
    word-break: break-all;
  }

  .harden-box {
    margin-top: 1.1rem;
    padding: 1rem;
    border-radius: 13px;
    background: rgba(37, 99, 235, 0.1);
    border: 1px solid rgba(59, 130, 246, 0.28);
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }
  .harden-head {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-weight: 600;
    font-size: 0.95rem;
  }
  .harden-box p {
    margin: 0;
    font-size: 0.82rem;
    color: #aeb9d1;
    line-height: 1.45;
  }
  .tag {
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 0.12rem 0.45rem;
    border-radius: 6px;
    background: #2563eb;
    color: white;
  }

  .steps {
    margin-top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .step {
    display: flex;
    align-items: flex-start;
    gap: 0.55rem;
    padding: 0.55rem 0.75rem;
    border-radius: 10px;
    background: rgba(9, 13, 24, 0.5);
    font-size: 0.86rem;
  }
  .step > div {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .step em {
    font-style: normal;
    color: #8695b3;
    font-size: 0.78rem;
  }
  .step.ok {
    color: #9fe6b6;
  }
  .step.skipped {
    color: #b7c0d6;
  }
  .step.failed {
    color: #f7a8a8;
    background: rgba(127, 29, 29, 0.2);
  }
  .harden-msg {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    margin-top: 0.3rem;
    padding: 0.75rem 0.9rem;
    border-radius: 11px;
    font-size: 0.86rem;
    font-weight: 500;
  }
  .harden-msg.ok {
    background: rgba(22, 101, 52, 0.24);
    border: 1px solid rgba(74, 222, 128, 0.32);
    color: #c7f9cc;
  }
  .harden-msg.err {
    background: rgba(127, 29, 29, 0.24);
    border: 1px solid rgba(248, 113, 113, 0.32);
    color: #fecaca;
  }

  .privacy {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    margin: 1.5rem 0 0;
    font-size: 0.76rem;
    color: #6f7b96;
  }
</style>
