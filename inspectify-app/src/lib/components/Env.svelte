<script lang="ts" generics="A extends ce_shell.Analysis">
  import type { ce_shell } from '$lib/api';
  import type { Io } from '$lib/io';
  import ValidationIndicator from './ValidationIndicator.svelte';

  export let io: Io<A>;
  const { results } = io;
  const notNull = <T,>(x: T | null): T => x!;
</script>

<div class="grid h-full w-full grid-cols-[min-content_1fr] grid-rows-[1fr_auto]">
  <div class="relative row-span-2 h-full w-[45ch] min-w-[20ch] max-w-[80vw] resize-x overflow-auto">
    <div class="absolute inset-0 grid">
      <slot name="input" />
    </div>
  </div>
  <div class="relative h-full">
    <div
      class="absolute inset-0 grid {$results.outputState == 'Stale'
        ? 'opacity-20 transition delay-[400ms] duration-1000'
        : 'transition'}"
    >
      {#if $results.output && $results.referenceOutput}
        <slot
          name="output"
          output={notNull($results.output)}
          referenceOutput={notNull($results.referenceOutput)}
        />
      {:else}
        <div class="absolute inset-0 grid place-items-center">
          <div class="text-2xl font-light italic">No output</div>
        </div>
      {/if}
    </div>
  </div>
  <ValidationIndicator {io} />
</div>
