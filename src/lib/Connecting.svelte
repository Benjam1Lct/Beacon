<script lang="ts">
  import { fade } from "svelte/transition";
  import Icon from "$lib/Icon.svelte";

  let {
    label,
    error,
    onCancel,
    onRetry,
  }: { label: string; error: string | null; onCancel: () => void; onRetry: () => void } = $props();
</script>

<div class="wrap" transition:fade={{ duration: 150 }}>
  {#if error}
    <div class="box">
      <span class="ic err"><Icon name="alert" size={30} /></span>
      <h1>Connexion impossible</h1>
      <p class="msg">{error}</p>
      <div class="actions">
        <button class="btn" onclick={onCancel}>Retour</button>
        <button class="btn primary" onclick={onRetry}>Réessayer</button>
      </div>
    </div>
  {:else}
    <div class="box">
      <span class="loader">
        <span class="ring"></span>
        <span class="brand"><Icon name="beacon" size={30} /></span>
      </span>
      <h1>Connexion à {label}</h1>
      <p class="dots">Récupération des métriques et des conteneurs<span>.</span><span>.</span><span>.</span></p>
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 2rem;
    color: #f4f4f5;
    font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
    background: rgba(5, 5, 6, 0.4);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
  }
  .box {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.7rem;
  }
  h1 {
    margin: 0.6rem 0 0;
    font-size: 1.3rem;
    letter-spacing: -0.02em;
  }

  .loader {
    position: relative;
    display: grid;
    place-items: center;
    width: 84px;
    height: 84px;
  }
  .ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 3px solid rgba(255, 255, 255, 0.12);
    border-top-color: #fff;
    animation: spin 0.9s linear infinite;
  }
  .brand {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 16px;
    background: #fff;
    color: #000;
    animation: pulse 1.6s ease-in-out infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes pulse {
    0%,
    100% {
      transform: scale(1);
      box-shadow: 0 0 0 0 rgba(255, 255, 255, 0.25);
    }
    50% {
      transform: scale(1.05);
      box-shadow: 0 0 0 12px rgba(255, 255, 255, 0);
    }
  }

  .dots {
    margin: 0;
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.88rem;
  }
  .dots span {
    animation: blink 1.4s infinite both;
  }
  .dots span:nth-child(2) {
    animation-delay: 0.2s;
  }
  .dots span:nth-child(3) {
    animation-delay: 0.4s;
  }
  @keyframes blink {
    0%,
    100% {
      opacity: 0.2;
    }
    50% {
      opacity: 1;
    }
  }

  .ic {
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: 18px;
  }
  .ic.err {
    background: rgba(248, 113, 113, 0.16);
    color: #fca5a5;
  }
  .msg {
    margin: 0;
    max-width: 420px;
    color: rgba(255, 255, 255, 0.6);
    font-size: 0.86rem;
  }
  .actions {
    display: flex;
    gap: 0.6rem;
    margin-top: 0.6rem;
  }
  .btn {
    padding: 0.65rem 1.2rem;
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.06);
    color: #f4f4f5;
    font-size: 0.9rem;
    cursor: pointer;
  }
  .btn.primary {
    border-color: transparent;
    background: #fff;
    color: #000;
  }
  .btn:hover {
    filter: brightness(1.1);
  }
</style>
