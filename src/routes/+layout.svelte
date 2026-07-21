<script lang="ts">
  let { children } = $props();
  // Couches de "vagues" empilées pour reproduire le wallpaper soyeux noir & blanc.
  const waves = Array.from({ length: 12 }, (_, i) => i);
</script>

<div class="backdrop" aria-hidden="true">
  <svg viewBox="0 0 1600 900" preserveAspectRatio="xMidYMid slice">
    <defs>
      <radialGradient id="glow" cx="20%" cy="32%" r="62%">
        <stop offset="0%" stop-color="#ffffff" stop-opacity="0.14" />
        <stop offset="55%" stop-color="#ffffff" stop-opacity="0.02" />
        <stop offset="100%" stop-color="#ffffff" stop-opacity="0" />
      </radialGradient>
      <linearGradient id="ridge" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0%" stop-color="#ffffff" stop-opacity="0.6" />
        <stop offset="42%" stop-color="#d6d9df" stop-opacity="0.3" />
        <stop offset="100%" stop-color="#ffffff" stop-opacity="0.02" />
      </linearGradient>
      <filter id="soft"><feGaussianBlur stdDeviation="1.3" /></filter>
    </defs>
    <rect width="1600" height="900" fill="#050505" />
    <ellipse cx="320" cy="280" rx="920" ry="520" fill="url(#glow)" />
    <g filter="url(#soft)" fill="none" stroke="url(#ridge)" stroke-width="1.4">
      {#each waves as i (i)}
        <path
          d="M -120 {430 + i * 27} C 260 {430 + i * 27} 470 {268 + i * 27} 800 {314 + i * 27} C 1082 {353 + i * 27} 1300 {522 + i * 27} 1720 {430 + i * 27}"
          opacity={Math.max(0.045, 0.24 - i * 0.017)}
        />
      {/each}
    </g>
  </svg>
  <!-- Remplace l'approximation par ta vraie image : dépose-la dans static/wallpaper.jpg -->
  <div class="photo"></div>
</div>

{@render children()}

<style>
  :global(*) {
    box-sizing: border-box;
  }
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    overflow-x: hidden;
    background: #050505;
  }
  :global(body) {
    font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
    color: #f4f4f5;
    -webkit-font-smoothing: antialiased;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: -1;
    overflow: hidden;
    background: #050505;
  }
  .backdrop svg {
    width: 100%;
    height: 100%;
    display: block;
  }
  .photo {
    position: absolute;
    inset: 0;
    background-image: url("/wallpaper.jpg");
    background-size: cover;
    background-position: center;
  }
</style>
