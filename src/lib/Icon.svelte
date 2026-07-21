<script lang="ts" module>
  // Icônes SVG remplies (fill = currentColor -> s'adaptent en blanc/noir selon le thème).
  // Aucun emoji dans l'app : on passe toujours par ce composant.
  export const ICONS: Record<string, string> = {
    beacon:
      "M12 11c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2m6 2c0-3.31-2.69-6-6-6s-6 2.69-6 6c0 2.22 1.21 4.15 3 5.19l1-1.74c-1.19-.7-2-1.97-2-3.45 0-2.21 1.79-4 4-4s4 1.79 4 4c0 1.48-.81 2.76-2 3.45l1 1.74c1.79-1.04 3-2.97 3-5.19m-6-10C6.48 3 2 7.48 2 13c0 3.7 2.01 6.92 4.99 8.65l1-1.73C5.61 18.53 4 15.96 4 13c0-4.42 3.58-8 8-8s8 3.58 8 8c0 2.96-1.61 5.53-4 6.92l1 1.73c2.99-1.73 5-4.95 5-8.65C22 7.48 17.52 3 12 3",
    server:
      "M2 20h20v-4H2zM4 17h2v2H4zm-2-3h20v-4H2zm2-3h2v2H4zm-2-7v4h20V4zm4 3H4V5h2z",
    key: "M12.65 10A5.99 5.99 0 0 0 7 6c-3.31 0-6 2.69-6 6s2.69 6 6 6a5.99 5.99 0 0 0 5.65-4H17v4h4v-4h2v-4zM7 14c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2",
    lock: "M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2m-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2m3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1s3.1 1.39 3.1 3.1z",
    check: "M9 16.17 4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z",
    alert: "M1 21h22L12 2zm12-3h-2v-2h2zm0-4h-2v-4h2z",
    plus: "M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6z",
    trash:
      "M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6zM19 4h-3.5l-1-1h-5l-1 1H5v2h14z",
    folder: "M10 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8z",
    close:
      "M19 6.41 17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z",
    arrow: "M12 4l-1.41 1.41L16.17 11H4v2h12.17l-5.58 5.59L12 20l8-8z",
    spinner:
      "M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46A7.93 7.93 0 0 0 20 12c0-4.42-3.58-8-8-8m0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8L5.24 7.74A7.93 7.93 0 0 0 4 12c0 4.42 3.58 8 8 8v3l4-4-4-4z",
  };
</script>

<script lang="ts">
  let { name, size = 20, spin = false }: { name: string; size?: number; spin?: boolean } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="currentColor"
  class:spin
  aria-hidden="true"
>
  <path d={ICONS[name] ?? ""} />
</svg>

<style>
  svg {
    display: inline-block;
    vertical-align: middle;
    flex-shrink: 0;
  }
  .spin {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
