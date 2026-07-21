<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "$lib/Icon.svelte";
  import type { ProfileMeta } from "$lib/types";

  let {
    profile,
    onClose,
    onDisconnect,
    onTerminal,
  }: {
    profile: ProfileMeta;
    onClose: () => void;
    onDisconnect: () => void;
    onTerminal: () => void;
  } = $props();

  function copy(v: string) {
    navigator.clipboard?.writeText(v);
  }
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
  <div class="settings" role="dialog" aria-modal="true" transition:scale={{ duration: 200, start: 0.96, easing: quintOut }}>
    <div class="shead">
      <span class="sicon"><Icon name="settings" size={20} /></span>
      <h2>Réglages</h2>
      <button class="icon-btn" onclick={onClose}><Icon name="close" size={18} /></button>
    </div>

    <div class="rows">
      <div class="row"><span>Nom</span><strong>{profile.label}</strong></div>
      <div class="row"><span>Adresse</span><strong>{profile.username}@{profile.host}:{profile.port}</strong></div>
      <div class="row"><span>Authentification</span><strong>{profile.authKind === "key" ? "Clé SSH" : "Mot de passe"}</strong></div>
      {#if profile.hostKeyFp}
        <div class="row fp">
          <span>Clé d'hôte (épinglée)</span>
          <button class="fp-val" title="Copier" onclick={() => copy(profile.hostKeyFp ?? "")}>
            {profile.hostKeyFp}<Icon name="logs" size={13} />
          </button>
        </div>
      {/if}
    </div>

    <div class="actions">
      <button class="btn" onclick={onTerminal}><Icon name="terminal" size={16} /> Terminal SSH</button>
      <button class="btn danger" onclick={onDisconnect}><Icon name="logout" size={16} /> Déconnecter</button>
    </div>

    <p class="ver">Beacon 0.1.0 · 100 % local</p>
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
  .settings {
    width: 100%;
    max-width: 460px;
    padding: 1.4rem;
    border-radius: 20px;
    background: rgba(24, 24, 27, 0.94);
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.65);
    color: #e7ecf3;
  }
  .shead {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    margin-bottom: 1.2rem;
  }
  .shead h2 {
    margin: 0;
    font-size: 1.2rem;
    flex: 1;
  }
  .sicon {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: rgba(148, 163, 184, 0.22);
    color: #cbd5e1;
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

  .rows {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    margin-bottom: 1.2rem;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.7rem 0.2rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    font-size: 0.88rem;
  }
  .row span {
    color: rgba(255, 255, 255, 0.5);
  }
  .row strong {
    text-align: right;
    word-break: break-word;
  }
  .row.fp {
    flex-direction: column;
    align-items: stretch;
    gap: 0.4rem;
  }
  .fp-val {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.5rem 0.6rem;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 9px;
    background: rgba(0, 0, 0, 0.4);
    color: rgba(255, 255, 255, 0.75);
    font-family: ui-monospace, monospace;
    font-size: 0.74rem;
    cursor: pointer;
    word-break: break-all;
    text-align: left;
  }
  .fp-val:hover {
    background: rgba(0, 0, 0, 0.55);
  }
  .fp-val :global(svg) {
    flex-shrink: 0;
    margin-left: auto;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .btn {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    padding: 0.7rem;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.06);
    color: #e7ecf3;
    font-size: 0.9rem;
    cursor: pointer;
  }
  .btn:hover {
    background: rgba(255, 255, 255, 0.13);
  }
  .btn.danger {
    border-color: rgba(220, 38, 38, 0.35);
    color: #fca5a5;
  }
  .btn.danger:hover {
    background: rgba(220, 38, 38, 0.16);
  }
  .ver {
    text-align: center;
    margin: 1.2rem 0 0;
    font-size: 0.74rem;
    color: rgba(255, 255, 255, 0.35);
  }
</style>
