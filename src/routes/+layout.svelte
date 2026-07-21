<script lang="ts">
  let { children } = $props();
</script>

<div class="backdrop" aria-hidden="true"></div>

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

  /* Wallpaper : ton image bg-black.jpg (static/), fond noir en repli. */
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: -1;
    background-color: #050505;
    background-image: url("/bg-black.jpg");
    background-size: cover;
    background-position: center;
    background-repeat: no-repeat;
  }
</style>
