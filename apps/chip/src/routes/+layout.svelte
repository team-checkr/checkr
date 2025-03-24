<script lang="ts">
  import '../app.css';

  interface Props {
    children?: import('svelte').Snippet;
  }

  let { children }: Props = $props();
</script>

<div class="grid min-h-screen grid-rows-[auto_1fr] bg-slate-800">
  {@render children?.()}
</div>
