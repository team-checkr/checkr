<script lang="ts">
  import { mirage } from 'ayu';
  import { onMount } from 'svelte';
  import type { Network } from 'vis-network/esnext';

  interface Props {
    dot: string;
  }

  let { dot }: Props = $props();

  let container: HTMLDivElement | undefined = $state();
  let network: Network | undefined = $state();

  let redraw = $derived(async () => {
    let preDot = dot;
    const vis = await import('vis-network/esnext');
    if (preDot != dot) return;
    const data = vis.parseDOTNetwork(dot);

    if (network) {
      network.setData(data);
    } else {
      if (!container) return;

      network = new vis.Network(container, data, {
        // interaction: { zoomView: false },
        nodes: {
          color: {
            background: mirage.ui.fg.hex(),
            border: mirage.ui.fg.hex(),
            highlight: mirage.ui.fg.brighten(1).hex(),
            // background: '#666666',
            // border: '#8080a0',
            // highlight: '#80a0ff',
          },
          font: {
            color: 'white',
          },
          borderWidth: 1,
          shape: 'circle',
          size: 30,
        },
        edges: {
          // color: '#D0D0FF',
          color: mirage.syntax.constant.hex(),
          font: {
            color: 'white',
            strokeColor: '#200020',
            face: 'Menlo, Monaco, "Courier New", monospace',
          },
        },
        autoResize: true,
      });
    }
  });

  onMount(() => {
    if (!container) return;
    const observer = new ResizeObserver(() => {
      requestAnimationFrame(() => {
        if (network) {
          network.fit({ animation: false, maxZoomLevel: 20 });
          network.redraw();
        }
      });
    });
    observer.observe(container);
    return () => observer?.disconnect();
  });

  onMount(() => {
    redraw();
  });

  $effect(() => {
    dot && network && redraw();
  });
</script>

<div class="relative h-full w-full">
  <div class="absolute inset-0" bind:this={container}></div>
</div>
