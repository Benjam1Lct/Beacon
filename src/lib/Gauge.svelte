<script lang="ts">
  let {
    value,
    label,
    sublabel = "",
  }: { value: number; label: string; sublabel?: string } = $props();

  const R = 42;
  const CIRC = 2 * Math.PI * R;

  const clamped = $derived(Math.max(0, Math.min(100, value)));
  const offset = $derived(CIRC * (1 - clamped / 100));
  // Même teal que le graphe réseau, rouge au seuil critique (>=90%).
  const color = $derived(clamped >= 90 ? "#f87171" : "#2dd4bf");
  // Point indicateur à l'extrémité de l'arc.
  const dotAngle = $derived(((-90 + (clamped / 100) * 360) * Math.PI) / 180);
  const dotX = $derived(50 + R * Math.cos(dotAngle));
  const dotY = $derived(50 + R * Math.sin(dotAngle));
</script>

<div class="gauge">
  <svg viewBox="0 0 100 100">
    <circle class="track" cx="50" cy="50" r={R} />
    <circle
      class="prog"
      cx="50"
      cy="50"
      r={R}
      transform="rotate(-90 50 50)"
      style="stroke:{color};stroke-dasharray:{CIRC};stroke-dashoffset:{offset}"
    />
    <circle class="dot" cx={dotX} cy={dotY} r="4.5" style="fill:{color}" />
    <text class="val" x="50" y="47">{Math.round(clamped)}<tspan class="pct">%</tspan></text>
    <text class="lbl" x="50" y="64">{label}</text>
  </svg>
  {#if sublabel}<span class="sub">{sublabel}</span>{/if}
</div>

<style>
  .gauge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
  }
  svg {
    width: 100%;
    max-width: 128px;
  }
  .track {
    fill: none;
    stroke: rgba(255, 255, 255, 0.08);
    stroke-width: 9;
  }
  .prog {
    fill: none;
    stroke-width: 9;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.6s ease, stroke 0.4s ease;
  }
  .dot {
    transition: cx 0.6s ease, cy 0.6s ease, fill 0.4s ease;
  }
  .val {
    fill: #f2f5fa;
    font-size: 22px;
    font-weight: 700;
    text-anchor: middle;
    dominant-baseline: middle;
  }
  .pct {
    font-size: 12px;
    fill: #93a1bd;
  }
  .lbl {
    fill: #93a1bd;
    font-size: 9px;
    text-anchor: middle;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .sub {
    font-size: 0.76rem;
    color: #8695b3;
  }
</style>
