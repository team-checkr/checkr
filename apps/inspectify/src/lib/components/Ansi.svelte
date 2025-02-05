<script lang="ts">
  import type { inspectify } from '$lib/api';
  interface Props {
    spans: inspectify.endpoints.Span[];
  }

  let { spans }: Props = $props();
</script>

{#each spans as span}<span class="fg-{span.fg} bg-{span.bg}">{span.text}</span>{/each}

<style>
  /* Foregrounds */
  .fg-Black {
    color: var(--color-text-black);
  }
  .fg-Red {
    color: var(--color-red-500);
  }
  .fg-Green {
    color: var(--color-green-500);
  }
  .fg-Yellow {
    color: var(--color-yellow-500);
  }
  .fg-Blue {
    color: var(--color-blue-500);
  }
  .fg-Magenta {
    color: var(--color-purple-500);
  }
  .fg-Cyan {
    color: var(--color-cyan-500);
  }
  .fg-White {
    color: var(--color-white);
  }
  /* .fg-Default {} */
  .fg-BrightBlack {
    color: var(--color-black);
  }
  .fg-BrightRed {
    color: var(--color-red-300);
  }
  .fg-BrightGreen {
    color: var(--color-green-300);
  }
  .fg-BrightYellow {
    color: var(--color-yellow-300);
  }
  .fg-BrightBlue {
    color: var(--color-blue-300);
  }
  .fg-BrightMagenta {
    color: var(--color-purple-300);
  }
  .fg-BrightCyan {
    color: var(--color-cyan-300);
  }
  .fg-BrightWhite {
    color: var(--color-white);
  }

  /* Backgrounds */
  .bg-Black {
    background-color: var(--color-bg-black);
  }
  .bg-Red {
    background-color: var(--color-bg-red-500);
  }
  .bg-Green {
    background-color: var(--color-bg-green-500);
  }
  .bg-Yellow {
    background-color: var(--color-bg-yellow-500);
  }
  .bg-Blue {
    background-color: var(--color-bg-blue-500);
  }
  .bg-Magenta {
    background-color: var(--color-bg-purple-500);
  }
  .bg-Cyan {
    background-color: var(--color-bg-cyan-500);
  }
  .bg-White {
    background-color: var(--color-bg-white);
  }
</style>
