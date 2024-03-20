<script lang="ts">
  import { onMount } from 'svelte';

  let container: HTMLElement;

  onMount(() => {
    const observer = new ResizeObserver(() => {
      container.scrollIntoView({
        behavior: 'smooth',
        block: 'end',
        inline: 'end',
      });
    });
    observer.observe(container);
    return () => observer.disconnect();
  });
</script>

<div bind:this={container} class="overflow-auto">
  <pre class="p-3 [overflow-anchor:none]"><code><slot /></code></pre>
  <div class="[overflow-anchor:auto]" />
</div>
