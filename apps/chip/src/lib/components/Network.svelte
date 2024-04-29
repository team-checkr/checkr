<script lang="ts">
  import { mirage } from 'ayu';
  import { onMount } from 'svelte';
  import type { Network } from 'vis-network/esnext';

  export let dot: string;
  export let hoveredNode: string | null = null;
  export let highlight: string[] = [];

  let container: HTMLDivElement;
  let network: Network | void;

  $: redraw = async () => {
    let preDot = dot;
    const vis = await import('vis-network/esnext');
    if (preDot != dot && preDot) {
      return;
    }
    let data: any;
    try {
      data = vis.parseDOTNetwork(dot || 'digraph G {}');
    } catch (e) {
      console.warn(e);
      console.log({ dot });
    }

    if (network) {
      network.setData(data);
    } else {
      network = new vis.Network(container, data, {
        interaction: { zoomView: true, hover: true, hoverConnectedEdges: false },
        nodes: {
          color: {
            background: mirage.ui.fg.hex(),
            border: mirage.ui.fg.hex(),
            highlight: mirage.syntax.keyword.hex(),
            hover: mirage.syntax.keyword.hex(),
            // background: '#666666',
            // border: '#8080a0',
            // highlight: '#80a0ff',
          },
          font: {
            color: 'white',
          },
          borderWidth: 1,
          shape: 'box',
          size: 30,
        },
        edges: {
          // color: '#D0D0FF',
          color: mirage.syntax.constant.hex(),
          hoverWidth: 0,
          selectionWidth: 0,
          font: {
            color: 'white',
            strokeColor: '#200020',
            face: 'Menlo, Monaco, "Courier New", monospace',
          },
        },
        autoResize: true,
        physics: {
          enabled: true,
          solver: 'forceAtlas2Based',
          stabilization: {
            enabled: false, // This is here just to see what's going on from the very beginning.
          },
        },
      });
    }
  };
  $: if (network) {
    network.on('hoverNode', (e: { node: string }) => {
      hoveredNode = e.node;
    });
    network.on('blurNode', (e: { node: string }) => {
      if (hoveredNode == e.node) hoveredNode = null;
    });
  }
  $: if (network) {
    try {
      network.unselectAll();
      network.setSelection({ nodes: highlight });
    } catch (e) {
      console.warn(e);
    }
  }

  onMount(() => {
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

  $: {
    dot && network && redraw();
  }
</script>

<div class="relative h-full w-full">
  <div class="absolute inset-0" bind:this={container}></div>
</div>
