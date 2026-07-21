<script lang="ts">
  import { sshTestConnection } from "$lib/api";
  import type { AuthInput, SshProfile, SshResult } from "$lib/types";

  let host = $state("");
  let port = $state(22);
  let username = $state("root");
  let authKind = $state<"key" | "password">("key");
  let keyPath = $state("");
  let passphrase = $state("");
  let password = $state("");

  let testing = $state(false);
  let result = $state<SshResult | null>(null);
  let error = $state<string | null>(null);

  const canSubmit = $derived(
    host.trim() !== "" &&
      username.trim() !== "" &&
      (authKind === "key" ? keyPath.trim() !== "" : password !== ""),
  );

  async function testConnection(event: Event) {
    event.preventDefault();
    testing = true;
    result = null;
    error = null;

    const auth: AuthInput =
      authKind === "key"
        ? { kind: "key", path: keyPath.trim(), passphrase: passphrase || null }
        : { kind: "password", password };

    const profile: SshProfile = { host: host.trim(), port, username: username.trim(), auth };

    try {
      result = await sshTestConnection(profile);
    } catch (e) {
      error = typeof e === "string" ? e : String(e);
    } finally {
      testing = false;
    }
  }
</script>

<main>
  <div class="card">
    <header>
      <div class="logo">🔦</div>
      <h1>Beacon</h1>
      <p class="tagline">Connecte-toi à ton serveur — VPS ou machine locale.</p>
    </header>

    <form onsubmit={testConnection}>
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
        <button
          type="button"
          class:active={authKind === "key"}
          onclick={() => (authKind = "key")}>Clé SSH</button
        >
        <button
          type="button"
          class:active={authKind === "password"}
          onclick={() => (authKind = "password")}>Mot de passe</button
        >
      </div>

      {#if authKind === "key"}
        <label>
          <span>Chemin de la clé privée</span>
          <input bind:value={keyPath} placeholder="C:\Users\toi\.ssh\id_ed25519" autocomplete="off" />
        </label>
        <label>
          <span>Passphrase <em>(si la clé en a une)</em></span>
          <input type="password" bind:value={passphrase} autocomplete="off" />
        </label>
      {:else}
        <label>
          <span>Mot de passe <em>(bootstrap uniquement, non enregistré)</em></span>
          <input type="password" bind:value={password} autocomplete="off" />
        </label>
      {/if}

      <button class="submit" type="submit" disabled={!canSubmit || testing}>
        {testing ? "Connexion…" : "Tester la connexion"}
      </button>
    </form>

    {#if result}
      <div class="feedback ok">
        <strong>✅ Connecté</strong>
        <code>{result.stdout.trim()}</code>
      </div>
    {/if}
    {#if error}
      <div class="feedback err">
        <strong>⚠️ Échec</strong>
        <span>{error}</span>
      </div>
    {/if}

    <p class="privacy">🔒 100 % local — aucune donnée ne quitte ta machine, sauf la connexion à ton serveur.</p>
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
  }

  main {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 2rem 1rem;
    font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
    color: #e7ecf3;
    background: radial-gradient(1200px 600px at 50% -10%, #1b2a4a 0%, #0c1220 55%, #080b14 100%);
  }

  .card {
    width: 100%;
    max-width: 460px;
    background: rgba(20, 27, 44, 0.72);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    padding: 2rem;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(12px);
  }

  header {
    text-align: center;
    margin-bottom: 1.5rem;
  }
  .logo {
    font-size: 2.4rem;
  }
  h1 {
    margin: 0.2rem 0 0;
    font-size: 1.7rem;
    letter-spacing: -0.02em;
  }
  .tagline {
    margin: 0.35rem 0 0;
    color: #93a1bd;
    font-size: 0.9rem;
  }

  form {
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
    padding: 0.7rem 0.85rem;
    border-radius: 11px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(9, 13, 24, 0.65);
    color: #f2f5fa;
    font-size: 0.95rem;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.25);
  }

  .auth-toggle {
    display: flex;
    gap: 0.4rem;
    background: rgba(9, 13, 24, 0.6);
    padding: 0.3rem;
    border-radius: 12px;
    margin-top: 0.2rem;
  }
  .auth-toggle button {
    flex: 1;
    padding: 0.55rem;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: #aeb9d1;
    font-size: 0.88rem;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .auth-toggle button.active {
    background: #2563eb;
    color: white;
  }

  .submit {
    margin-top: 0.5rem;
    padding: 0.8rem;
    border: none;
    border-radius: 12px;
    background: linear-gradient(180deg, #3b82f6, #2563eb);
    color: white;
    font-size: 0.98rem;
    font-weight: 600;
    cursor: pointer;
    transition: filter 0.15s, opacity 0.15s;
  }
  .submit:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .submit:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .feedback {
    margin-top: 1.1rem;
    padding: 0.9rem 1rem;
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.88rem;
  }
  .feedback.ok {
    background: rgba(22, 101, 52, 0.25);
    border: 1px solid rgba(74, 222, 128, 0.35);
  }
  .feedback.err {
    background: rgba(127, 29, 29, 0.25);
    border: 1px solid rgba(248, 113, 113, 0.35);
  }
  .feedback code {
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 0.8rem;
    color: #c7f9cc;
    word-break: break-word;
  }

  .privacy {
    margin: 1.4rem 0 0;
    text-align: center;
    font-size: 0.76rem;
    color: #6f7b96;
  }
</style>
