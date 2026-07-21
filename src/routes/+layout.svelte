<script lang="ts">
  let { children } = $props();
</script>

{@render children()}

<style>
  :global(*) {
    box-sizing: border-box;
  }
  :global(html) {
    height: 100%;
    background: #050505;
  }
  :global(body) {
    margin: 0;
    min-height: 100%;
    overflow-x: hidden;
    font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
    color: #f4f4f5;
    -webkit-font-smoothing: antialiased;
    /* Wallpaper (static/bg-black.jpg) appliqué au fond de page : rendu fiable dans WebView2. */
    background-color: #050505;
    background-image: url("/bg-black.jpg");
    background-size: cover;
    background-position: center;
    background-repeat: no-repeat;
    background-attachment: fixed;
  }
</style>
