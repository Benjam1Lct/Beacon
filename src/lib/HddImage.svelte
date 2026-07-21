<script lang="ts">
  let { size = 56 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
  <defs>
    <linearGradient id="hdd-body" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#e2e5ea" />
      <stop offset="0.5" stop-color="#b9bdc6" />
      <stop offset="1" stop-color="#8b909b" />
    </linearGradient>
    <radialGradient id="hdd-platter" cx="0.4" cy="0.35" r="0.8">
      <stop offset="0" stop-color="#5b6068" />
      <stop offset="0.7" stop-color="#31343a" />
      <stop offset="1" stop-color="#1c1e22" />
    </radialGradient>
  </defs>
  <!-- Corps -->
  <rect x="6" y="7" width="52" height="50" rx="5" fill="url(#hdd-body)" stroke="#6e737d" stroke-width="0.6" />
  <rect x="9" y="10" width="46" height="44" rx="3" fill="#a7abb4" opacity="0.35" />
  <!-- Plateau -->
  <circle cx="28" cy="33" r="18" fill="url(#hdd-platter)" />
  <circle cx="28" cy="33" r="18" fill="none" stroke="#6b7076" stroke-width="0.5" opacity="0.5" />
  <circle cx="28" cy="33" r="6" fill="#15171a" />
  <circle cx="28" cy="33" r="2.2" fill="#8b9096" />
  <!-- Bras de lecture -->
  <path d="M52 13 L33 30 l3.4 3.8 L55.5 16.5 Z" fill="#c2c6cd" stroke="#6e737d" stroke-width="0.5" />
  <circle cx="53" cy="14" r="3.4" fill="#9aa0a8" stroke="#6e737d" stroke-width="0.5" />
  <!-- Vis -->
  <circle cx="10" cy="11" r="1.4" fill="#7d828c" />
  <circle cx="54" cy="53" r="1.4" fill="#7d828c" />
</svg>
