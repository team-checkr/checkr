<!-- <script lang="ts">
  import { mirage } from 'ayu';
  import { onMount } from 'svelte';
  import type { Network } from 'vis-network/esnext';

  interface Props {
    dot: string;
  }

  let { dot }: Props = $props();

  let container: HTMLDivElement | undefined = $state();
  let network: Network | undefined = $state();

  async function redraw() {
    let preDot = dot;
    const vis = await import('vis-network/esnext');
    if (preDot != dot) return;

    const data = vis.parseDOTNetwork(dot);

    // Patch nodes — vis global options overwrite DOT attributes, so we fix them here
    data.nodes = data.nodes.map((node: any) => {
      if (node.id === '__start') {
        // Hide the invisible start node completely
        return {
          ...node,
          shape: 'dot',
          size: 0,
          color: { background: 'transparent', border: 'transparent', highlight: 'transparent' },
          borderWidth: 0,
        };
      }
      if (node.shape === 'doublecircle') {
        // Accept state — highlight with a colored border
        return {
          ...node,
          shape: 'doublecircle',
          borderWidth: 5,
          color: {
            background: mirage.ui.fg.hex(),
            border: mirage.syntax.constant.hex(),
            highlight: mirage.ui.fg.brighten(1).hex(),
          },
          font: { color: 'white' },
        };
      }
      // Normal state
      return {
        ...node,
        shape: 'circle',
        borderWidth: 1,
        color: {
          background: mirage.ui.fg.hex(),
          border: mirage.ui.fg.hex(),
          highlight: mirage.ui.fg.brighten(1).hex(),
        },
        font: { color: 'white' },
      };
    });

    if (network) {
      network.setData(data);
    } else {
      if (!container) return;
      network = new vis.Network(container, data, {
        edges: {
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
  }

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
    return () => observer.disconnect();
  });

  onMount(() => {
    redraw();
  });

  $effect(() => {
    dot && redraw();
  });
</script>

<div class="relative h-full w-full">
  <div class="absolute inset-0" bind:this={container}></div>
</div> -->

<script lang="ts">
  import { Graphviz } from '@hpcc-js/wasm-graphviz';

  interface Props { dot: string; }
  let { dot }: Props = $props();

  let graphviz: Graphviz | null = null;
  let svg = $state('');

  // Load graphviz once
  Graphviz.load().then(g => {
    graphviz = g;
    svg = graphviz.dot(dot);
  });

  // Only re-render when dot changes
  $effect(() => {
    if (graphviz && dot) {
      svg = graphviz.dot(dot);
    }
  });
</script>

<div class="h-full w-full">{@html svg}</div>