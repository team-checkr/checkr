<script lang="ts">
  interface Props {
    title: string;
    children?: import('svelte').Snippet;
  }

  let { title, children }: Props = $props();
</script>

<div class="px-2 py-0.5 text-sm font-bold">{title}</div>
<div class="w-full font-mono">
  {@render children?.()}
</div>
