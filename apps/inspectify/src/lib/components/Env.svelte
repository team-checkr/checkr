<script lang="ts" generics="A extends ce_shell.Analysis">
  import { showReference } from '$lib/jobs.svelte';

  import { crossfade } from 'svelte/transition';
  import { quintOut } from 'svelte/easing';
  import type { ce_shell } from '$lib/api';
  import type { Input, Io, Meta, Output } from '$lib/io.svelte';
  import Ansi from './Ansi.svelte';
  import JobTabs from './JobTabs.svelte';
  import TrackingScroll from './TrackingScroll.svelte';
  import ValidationIndicator from './ValidationIndicator.svelte';

  interface Props {
    io: Io<A>;
    inputView?: import('svelte').Snippet;
    outputView?: import('svelte').Snippet<
      [
        {
          input: Input<A>;
          meta: Meta<A>;
          output: Output<A>;
          referenceOutput: Output<A>;
        },
      ]
    >;
  }

  let { io, inputView: input, outputView: output }: Props = $props();
  const notNull = <T,>(x: T | null): T => x!;

  let results = $derived(showReference.show ? io.reference : io.results);

  let latestJob = $derived(results.job);
  let hideTabs = $state(true);

  const [send, receive] = crossfade({
    delay: 200,
    duration: 200,
    easing: quintOut,
  });
  const key = '123';
</script>

<div class="grid h-full w-full grid-cols-[min-content_1fr] grid-rows-[1fr_auto]">
  <div class="relative row-span-2 h-full w-[45ch] min-w-[20ch] max-w-[80vw] resize-x overflow-auto">
    <div class="absolute inset-0 grid">
      {@render input?.()}
    </div>
  </div>
  <div class="relative h-full">
    <div
      class="absolute inset-0 grid overflow-auto {results.outputState == 'Stale'
        ? 'opacity-20 transition delay-[400ms] duration-1000'
        : 'transition'}"
    >
      {#if results.output && results.referenceOutput}
        <div in:send={{ key }} out:receive={{ key }} class="grid">
          <div class="grid grid-rows-[1fr_auto]">
            <div class="relative">
              <div class="absolute inset-0 grid overflow-auto">
                {@render output?.({
                  input: results.input,
                  meta: notNull(io.meta),
                  output: notNull(results.output),
                  referenceOutput: notNull(results.referenceOutput),
                })}
              </div>
            </div>
            {#if latestJob}
              <div
                class="grid border-t {hideTabs ? 'grid-rows-[1fr_auto]' : 'grid-rows-[30vh_auto]'}"
              >
                <JobTabs selectedJob={latestJob} canHide bind:hidden={hideTabs} />
              </div>
            {/if}
          </div>
        </div>
      {:else if latestJob?.state == 'Failed'}
        <div
          in:send={{ key }}
          out:receive={{ key }}
          class="absolute inset-0 grid grid-rows-[auto_1fr_auto] text-xs"
        >
          <div class="border-y bg-slate-900 p-2 text-xl font-light italic">Analysis failed</div>
          <div class="flex overflow-auto">
            <TrackingScroll>
              <Ansi spans={latestJob.spans} />
            </TrackingScroll>
          </div>
          {#if latestJob}
            <div
              class="grid border-t {hideTabs ? 'grid-rows-[1fr_auto]' : 'grid-rows-[30vh_auto]'}"
            >
              <JobTabs selectedJob={latestJob} canHide bind:hidden={hideTabs} />
            </div>
          {/if}
        </div>
      {:else}
        <div
          in:send={{ key }}
          out:receive={{ key }}
          class="absolute inset-0 grid place-items-center"
        >
          <div class="text-2xl font-light italic">
            {#if results.outputState == 'Current'}
              No output
            {:else}
              Loading...
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
  <div class="grid">
    {#if !showReference.show}
      <ValidationIndicator {io} />
    {/if}
  </div>
</div>
