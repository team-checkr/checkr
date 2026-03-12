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

<div class="h-full w-full">{@html svg}</div>