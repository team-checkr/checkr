<script lang="ts" generics="A extends ce_shell.Analysis">
  import type { ce_shell } from '$lib/api';
  import { jobsStore } from '$lib/events';
  import type { Io } from '$lib/io';
  import JobTabs from './JobTabs.svelte';
  import ValidationIndicator from './ValidationIndicator.svelte';

  export let io: Io<A>;
  const { results } = io;
  const notNull = <T,>(x: T | null): T => x!;

  $: latestJob = $results.latestJobId ? $jobsStore[$results.latestJobId] : null;
  let hideTabs = true;
</script>

<div class="grid h-full w-full grid-cols-[min-content_1fr] grid-rows-[1fr_auto]">
  <div class="relative row-span-2 h-full w-[45ch] min-w-[20ch] max-w-[80vw] resize-x overflow-auto">
    <div class="absolute inset-0 grid">
      <slot name="input" />
    </div>
  </div>
  <div class="relative h-full">
    <div
      class="absolute inset-0 grid overflow-auto {$results.outputState == 'Stale'
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
  <div class="grid {hideTabs ? 'grid-rows-[1fr_auto]' : 'grid-rows-[30vh_auto]'}">
    {#if $latestJob}
      <div class="grid border-t">
        <JobTabs selectedJob={$latestJob} canHide bind:hidden={hideTabs} />
      </div>
    {/if}
    <ValidationIndicator {io} />
  </div>
</div>
