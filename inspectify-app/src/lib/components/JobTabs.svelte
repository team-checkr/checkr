<script lang="ts">
  import { type Tab, currentTab, tabs } from '$lib/jobs';
  import Ansi from '$lib/components/Ansi.svelte';
  import JsonView from './JSONView.svelte';
  import TrackingScroll from './TrackingScroll.svelte';
  import type { ce_core } from '$lib/api';
  import type { Job } from '$lib/events';

  export let selectedJob: Job;
  export let canHide = false;
  export let hidden = canHide;

  $: if (selectedJob?.kind.kind == 'Compilation') {
    $currentTab = 'Output';
  }
  $: isDisabled = (tab: Tab) =>
    selectedJob ? tab != 'Output' && selectedJob.kind.kind == 'Compilation' : true;

  const validationTypeSymbols: Record<ce_core.ValidationResult['type'], string> = {
    CorrectTerminated: '✅',
    CorrectNonTerminated: '✅',
    Mismatch: '❌',
    TimeOut: '⚠️',
  };
</script>

<div class="grid grid-rows-[auto_1fr]">
  <div class="flex text-sm">
    {#each tabs as tab}
      <button
        class="flex flex-1 items-center justify-center px-2 py-1 transition disabled:opacity-50 {(!hidden &&
          tab == $currentTab) ||
        isDisabled(tab)
          ? 'bg-slate-700'
          : 'hover:bg-slate-800'}"
        on:click={() => {
          if (canHide && $currentTab == tab) {
            hidden = !hidden;
          } else {
            hidden = false;
            $currentTab = tab;
          }
        }}
        disabled={isDisabled(tab)}
      >
        {tab}
        {#if tab == 'Validation'}
          <span class="w-6">
            {selectedJob.analysis_data?.validation?.type
              ? validationTypeSymbols[selectedJob.analysis_data?.validation?.type]
              : '…'}
          </span>
        {/if}
      </button>
    {/each}
  </div>
  {#if !hidden}
    <div class="relative self-stretch bg-slate-900 text-xs">
      <div class="absolute inset-0 flex overflow-auto">
        {#if $currentTab == 'Output'}
          <TrackingScroll>
            <Ansi spans={selectedJob.spans} />
          </TrackingScroll>
        {:else if $currentTab == 'Input JSON' && selectedJob.kind.kind == 'Analysis'}
          <JsonView json={selectedJob.kind.data.json} />
          <div class="[overflow-anchor:auto]" />
        {:else if $currentTab == 'Output JSON'}
          {#if selectedJob.analysis_data?.output}
            <JsonView json={selectedJob.analysis_data.output.json} />
          {:else}
            <div class="p-2">
              <div class="italic text-red-500">Failed to parse JSON</div>
              {#if selectedJob.stdout.length > 0}
                <pre class="p-3 [overflow-anchor:none]"><code>{selectedJob.stdout}</code></pre>
              {:else}
                <pre class="p-3 italic text-gray-400 [overflow-anchor:none]"><code
                    >&lt;stdout was empty&gt;</code
                  ></pre>
              {/if}
            </div>
          {/if}
          <div class="[overflow-anchor:auto]" />
        {:else if $currentTab == 'Reference Output'}
          <JsonView json={selectedJob.analysis_data?.reference_output?.json} />
          <div class="[overflow-anchor:auto]" />
        {:else if $currentTab == 'Validation'}
          <JsonView json={selectedJob.analysis_data?.validation} />
          <div class="[overflow-anchor:auto]" />
        {/if}
      </div>
    </div>
  {/if}
</div>
