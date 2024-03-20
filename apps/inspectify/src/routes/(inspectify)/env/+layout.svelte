<script lang="ts">
  import Ansi from '$lib/components/Ansi.svelte';
  import JobPane from '$lib/components/JobPane.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import TrackingScroll from '$lib/components/TrackingScroll.svelte';
  import { compilationStatusStore, jobsStore } from '$lib/events';
  import { showStatus } from '$lib/jobs';

  import ArrowPath from '~icons/heroicons/arrow-path';
  import Fire from '~icons/heroicons/fire';

  $: compilationJob =
    typeof $compilationStatusStore?.id == 'number' ? $jobsStore[$compilationStatusStore.id] : null;
  $: compilationError = $compilationStatusStore?.state == 'Failed';
</script>

<div class="relative grid grid-rows-[1fr_auto]">
  <main class="relative grid h-full">
    <div class="absolute inset-0 grid">
      <slot />
    </div>
  </main>

  {#if $showStatus}
    <div class="h-[35vh]">
      <JobPane />
    </div>
  {/if}

  {#if $compilationStatusStore && $compilationStatusStore.state != 'Succeeded'}
    <div class="absolute inset-0 mt-20 grid items-start justify-center">
      <div
        class="grid h-[60vh] w-[50em] grid-rows-[auto_1fr] overflow-hidden rounded-lg bg-slate-600 shadow-xl"
      >
        <div
          class="flex items-center justify-between px-3 py-1 transition {compilationError
            ? 'bg-red-600'
            : 'bg-slate-500'}"
        >
          <h2 class="text-xl font-light italic">Compilation</h2>
          <div>
            {#if compilationError}
              <Fire class="text-lg text-red-200" />
            {:else if $compilationStatusStore.state == 'Running'}
              <ArrowPath class="animate-spin text-lg text-white" />
            {:else}
              <span class="text-lg text-white">...</span>
            {/if}
          </div>
        </div>
        <div class="relative h-full w-full">
          <div class="absolute inset-0 overflow-auto text-sm">
            {#if $compilationJob}
              <TrackingScroll>
                <Ansi spans={$compilationJob?.spans} />
              </TrackingScroll>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<StatusBar />
