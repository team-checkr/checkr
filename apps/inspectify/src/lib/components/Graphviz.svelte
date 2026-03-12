<script lang="ts">
  import { Graphviz } from '@hpcc-js/wasm-graphviz';

  interface Props { dot: string; }
  let { dot }: Props = $props();

  let graphviz: Graphviz | null = $state(null);
  let svg = $state('');

  // Load graphviz once
  Graphviz.load().then(g => {
    graphviz = g;
  });

  // Only re-render when dot changes
  $effect(() => {
    const currentDot = dot;
    if (graphviz && currentDot) {
      svg = graphviz.dot(dot);
    }
  });
</script>

<div class="h-full w-full flex items-center justify-center">{@html svg}</div>

<style>
  div :global(svg) {
    max-width: 100%;
    max-height: 100%;
    height: auto;
    width: auto;
    background: transparent;
  }

  div :global(polygon[fill="black"]) {
    fill: white;
    stroke: none;
  }

  div :global(ellipse),
  div :global(path) {
    stroke: white;
  }

  div :global(text) {
    fill: white;
  }

  div :global(polygon[fill="white"]),
  div :global(ellipse[fill="white"]) {
    fill: transparent;
  }
</style>