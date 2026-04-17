<script lang="ts">
  import { Graphviz } from '@hpcc-js/wasm-graphviz';
  import { mirage } from 'ayu';

  interface Props {
    dot: string;
  }
  let { dot }: Props = $props();

  let graphviz: Graphviz | null = $state(null);
  let svg = $state('');

  function enrichDot(dotStr: string): string {
    let enriched = dotStr;
    let initialNode: string | null = null;

    // First pass: handle nodes that are both accepting and initial
    enriched = enriched.replace(
      /(\w+)\s*\[(.*?isAccepting=true.*?isInitial=true.*?)\];/g,
      (match, node, attrs) => {
        initialNode = node;
        let cleanAttrs = attrs
          .replace(/isAccepting=true/g, '')
          .replace(/isInitial=true/g, '')
          .trim();
        cleanAttrs = cleanAttrs.replace(/^,\s*/, '').replace(/\s*,$/, '').trim();

        if (cleanAttrs) {
          return `${node} [${cleanAttrs}, shape=doublecircle, class="accepting"];`;
        } else {
          return `${node} [shape=doublecircle, class="accepting"];`;
        }
      },
    );

    // Handle nodes that are both accepting and initial (alternate order)
    enriched = enriched.replace(
      /(\w+)\s*\[(.*?isInitial=true.*?isAccepting=true.*?)\];/g,
      (match, node, attrs) => {
        initialNode = node;
        let cleanAttrs = attrs
          .replace(/isAccepting=true/g, '')
          .replace(/isInitial=true/g, '')
          .trim();
        cleanAttrs = cleanAttrs.replace(/^,\s*/, '').replace(/\s*,$/, '').trim();

        if (cleanAttrs) {
          return `${node} [${cleanAttrs}, shape=doublecircle, class="accepting"];`;
        } else {
          return `${node} [shape=doublecircle, class="accepting"];`;
        }
      },
    );

    // Extract and convert accepting states to double circles
    enriched = enriched.replace(/(\w+)\s*\[(.*?isAccepting=true.*?)\];/g, (match, node, attrs) => {
      let cleanAttrs = attrs.replace(/isAccepting=true/g, '').trim();
      cleanAttrs = cleanAttrs.replace(/^,\s*/, '').replace(/\s*,$/, '').trim();

      if (cleanAttrs) {
        return `${node} [${cleanAttrs}, shape=doublecircle, class="accepting"];`;
      } else {
        return `${node} [shape=doublecircle, class="accepting"];`;
      }
    });

    // Extract initial node and add __start node with arrow
    enriched = enriched.replace(/(\w+)\s*\[(.*?isInitial=true.*?)\];/g, (match, node, attrs) => {
      initialNode = node;
      let cleanAttrs = attrs.replace(/isInitial=true/g, '').trim();
      cleanAttrs = cleanAttrs.replace(/^,\s*/, '').replace(/\s*,$/, '').trim();

      if (cleanAttrs) {
        return `${node} [${cleanAttrs}, shape=circle];`;
      } else {
        return `${node} [shape=circle];`;
      }
    });

    // Handle nodes with attributes - replace any shape with circle, or add circle if missing
    enriched = enriched.replace(/(\w+)\s*\[([^\]]*)\];/g, (match, node, attrs) => {
      if (node === '__start') return match;
      // If shape=doublecircle is already there, don't change it
      if (attrs.includes('shape=doublecircle')) return match;
      // If class=accepting, preserve it (for accepting nodes)
      if (attrs.includes('class=accepting')) return match;
      // If it has a different shape, replace it with circle
      if (attrs.includes('shape=')) {
        const cleanedAttrs = attrs.replace(/shape=\w+/g, 'shape=circle');
        return `${node} [${cleanedAttrs}];`;
      }
      // If no shape, add circle
      if (attrs.trim()) {
        return `${node} [${attrs}, shape=circle];`;
      } else {
        return `${node} [shape=circle];`;
      }
    });

    // Handle nodes without attributes - add them with shape=circle
    enriched = enriched.replace(/\n\s+(\w+)\s*;/g, (match, node) => {
      if (node === '__start') return match;
      return `\n  ${node} [shape=circle];`;
    });

    // Add __start node and arrow if we found an initial state
    if (initialNode) {
      enriched = enriched.replace(
        /(digraph[^{]*{)/,
        `$1\n  rankdir=LR\n  __start [label="", shape=none]\n  __start -> ${initialNode}`,
      );
    }

    return enriched;
  }

  // Load graphviz once
  $effect(() => {
    Graphviz.load().then((g) => {
      graphviz = g;
    });
  });

  // Only re-render when dot changes
  $effect(() => {
    const currentDot = dot;
    if (graphviz && currentDot) {
      svg = graphviz.dot(enrichDot(currentDot));
    } else {
      svg = '';
    }
  });
</script>

<div class="flex h-full w-full items-center justify-center">{@html svg}</div>

<style>
  div :global(svg) {
    max-width: 100%;
    max-height: 100%;
    height: auto;
    width: auto;
    background: transparent;
  }

  /* Arrow heads - pink/magenta */
  div :global(polygon[fill='black']) {
    fill: #dfbfff;
    stroke: none;
  }

  /* Node circles - filled gray, no stroke by default */
  div :global(ellipse) {
    stroke: none !important;
    stroke-width: 0 !important;
    fill: #6b7a8f !important;
  }

  /* Accepting nodes (double circles) - all ellipses in group get green stroke with glow */
  div :global(g[class*='accepting'] ellipse) {
    stroke: #85cc95 !important;
    stroke-width: 1.5 !important;
    fill: #6b7a8f !important;
    filter: drop-shadow(0 0 4px #85cc95);
  }

  /* Second ellipse (inner circle) - make it transparent */
  div :global(g[class*='accepting'] ellipse + ellipse) {
    fill: transparent !important;
  }

  /* Edges - pink/magenta */
  div :global(path[stroke]) {
    stroke: #dfbfff !important;
    stroke-width: 1.5 !important;
  }

  /* Edge labels */
  div :global(text) {
    fill: white !important;
    font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
  }

  /* Remove default white fills */
  div :global(polygon[fill='white']),
  div :global(circle[fill='white']) {
    fill: transparent !important;
  }
</style>
