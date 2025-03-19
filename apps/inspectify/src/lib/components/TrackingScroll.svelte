<script lang="ts">
  import { onMount } from 'svelte';
  interface Props {
    children?: import('svelte').Snippet;
  }

  let { children }: Props = $props();

  let container: HTMLElement | undefined = $state();

  onMount(() => {
    if (!container) return;
    const observer = new ResizeObserver(() => {
      container?.scrollIntoView({
        behavior: 'smooth',
        block: 'end',
        inline: 'end',
      });
    });
    observer.observe(container);
    return () => observer.disconnect();
  });
</script>

<div bind:this={container} class="h-full overflow-auto">
  <pre class="p-3 [overflow-anchor:none]"><code>{@render children?.()}</code></pre>
  <div class="[overflow-anchor:auto]"></div>
</div>
