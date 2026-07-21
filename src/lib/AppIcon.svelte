<script lang="ts">
  // Icônes d'applications "natives" façon ZimaOS : badge dégradé + brillance + glyphe blanc.
  // Chaque icône a des id de gradient uniques (sinon collision entre SVG d'une même page).
  let { name, size = 60 }: { name: string; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 64 64" class="app-icon" aria-hidden="true">
  <defs>
    <linearGradient id="gl-{name}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#fff" stop-opacity="0.24" />
      <stop offset="0.5" stop-color="#fff" stop-opacity="0" />
    </linearGradient>
    {#if name === "appstore"}
      <linearGradient id="bg-{name}" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#5aa0ff" /><stop offset="1" stop-color="#2563eb" />
      </linearGradient>
    {:else if name === "files"}
      <linearGradient id="bg-{name}" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#44c6f5" /><stop offset="1" stop-color="#0284c7" />
      </linearGradient>
    {:else if name === "terminal"}
      <linearGradient id="bg-{name}" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#4b5563" /><stop offset="1" stop-color="#1b1f27" />
      </linearGradient>
    {:else if name === "containers"}
      <linearGradient id="bg-{name}" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#2ee6c7" /><stop offset="1" stop-color="#0d9488" />
      </linearGradient>
    {:else if name === "network"}
      <linearGradient id="bg-{name}" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#8b93ff" /><stop offset="1" stop-color="#6366f1" />
      </linearGradient>
    {:else}
      <linearGradient id="bg-{name}" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#93a3b8" /><stop offset="1" stop-color="#475569" />
      </linearGradient>
    {/if}
  </defs>

  <rect width="64" height="64" rx="15" fill="url(#bg-{name})" />
  <rect width="64" height="64" rx="15" fill="url(#gl-{name})" />

  {#if name === "appstore"}
    <g fill="#fff">
      <rect x="16" y="16" width="14" height="14" rx="4" opacity="0.95" />
      <rect x="34" y="16" width="14" height="14" rx="4" opacity="0.75" />
      <rect x="16" y="34" width="14" height="14" rx="4" opacity="0.75" />
      <rect x="34" y="34" width="14" height="14" rx="4" opacity="0.95" />
    </g>
  {:else if name === "files"}
    <path
      d="M15 20h11l3 4h20a3 3 0 0 1 3 3v17a3 3 0 0 1-3 3H15a3 3 0 0 1-3-3V23a3 3 0 0 1 3-3z"
      fill="#fff"
    />
    <path d="M15 25h34v3H15z" fill="#000" opacity="0.06" />
  {:else if name === "terminal"}
    <rect x="13" y="17" width="38" height="30" rx="4" fill="#0f1319" opacity="0.55" />
    <path d="M20 27l6 5-6 5" fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" />
    <path d="M31 39h9" stroke="#fff" stroke-width="3" stroke-linecap="round" />
  {:else if name === "containers"}
    <g fill="#fff">
      <rect x="14" y="26" width="11" height="14" rx="1.5" />
      <rect x="27" y="26" width="11" height="14" rx="1.5" opacity="0.9" />
      <rect x="40" y="26" width="11" height="14" rx="1.5" />
      <rect x="20.5" y="20" width="11" height="4.5" rx="1.5" opacity="0.85" />
      <rect x="33.5" y="20" width="11" height="4.5" rx="1.5" opacity="0.85" />
    </g>
    <path d="M12 42h40" stroke="#fff" stroke-width="2.5" stroke-linecap="round" opacity="0.9" />
  {:else if name === "network"}
    <g stroke="#fff" stroke-width="2.4" opacity="0.9">
      <line x1="22" y1="24" x2="42" y2="24" />
      <line x1="22" y1="24" x2="22" y2="40" />
      <line x1="42" y1="24" x2="32" y2="40" />
    </g>
    <g fill="#fff">
      <circle cx="22" cy="22" r="5.5" />
      <circle cx="42" cy="22" r="5.5" />
      <circle cx="22" cy="42" r="5.5" />
      <circle cx="32" cy="42" r="5.5" />
    </g>
  {:else}
    <path
      d="M32 22a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm0 6a4 4 0 1 1 0 8 4 4 0 0 1 0-8z"
      fill="#fff"
      opacity="0.15"
    />
    <path
      d="M50 34.9v-5.8l-4.4-1.1a14 14 0 0 0-1.4-3.4l2.3-3.9-4.1-4.1-3.9 2.3a14 14 0 0 0-3.4-1.4L34 10h-5.8l-1.1 4.4a14 14 0 0 0-3.4 1.4l-3.9-2.3-4.1 4.1 2.3 3.9a14 14 0 0 0-1.4 3.4L14 29.1v5.8l4.4 1.1a14 14 0 0 0 1.4 3.4l-2.3 3.9 4.1 4.1 3.9-2.3a14 14 0 0 0 3.4 1.4L30 54h5.8l1.1-4.4a14 14 0 0 0 3.4-1.4l3.9 2.3 4.1-4.1-2.3-3.9a14 14 0 0 0 1.4-3.4zM32 39.5a7.5 7.5 0 1 1 0-15 7.5 7.5 0 0 1 0 15z"
      fill="#fff"
    />
  {/if}
</svg>

<style>
  .app-icon {
    display: block;
    border-radius: 15px;
    box-shadow: 0 10px 22px rgba(0, 0, 0, 0.4);
  }
</style>
