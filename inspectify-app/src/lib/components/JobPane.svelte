<script lang="ts">
  import { derived } from 'svelte/store';
  import { driver } from '$lib/api';
  import { jobsListStore, jobsStore } from '$lib/events';
  import { selectedJobId } from '$lib/jobs';
  import JobTabs from './JobTabs.svelte';

  import EllipsisHorizontal from '~icons/heroicons/ellipsis-horizontal';
  import ArrowPath from '~icons/heroicons/arrow-path';
  import Check from '~icons/heroicons/check';
  import NoSymbol from '~icons/heroicons/no-symbol';
  import Fire from '~icons/heroicons/fire';
  import ExclamationTriangle from '~icons/heroicons/exclamation-triangle';

  export let showGroup = false;

  const icons: Record<driver.job.JobState, [typeof EllipsisHorizontal, string]> = {
    Queued: [EllipsisHorizontal, 'animate-pulse'],
    Running: [ArrowPath, 'animate-spin text-slate-400'],
    Succeeded: [Check, 'text-green-300'],
    Canceled: [NoSymbol, 'text-slate-400'],
    Failed: [Fire, 'text-red-300'],
    Warning: [ExclamationTriangle, 'text-yellow-300'],
  };

  const Icon = (state: driver.job.JobState) => icons[state][0];

  $: jobs = derived(
    $jobsListStore.map((id) => $jobsStore[id]),
    (jobs) => jobs,
  );
  $: filteredJobs = $jobs.filter((j) => j.state != 'Canceled');
  // $: filteredJobs = $jobs.filter((j) => j.state != 'Canceled');
  $: selectedJob = typeof $selectedJobId == 'number' ? $jobsStore[$selectedJobId] : null;
</script>

<div
  class="z-10 grid h-full {showGroup
    ? 'grid-cols-[25ch_1fr]'
    : 'grid-cols-[20ch_1fr]'} border-t bg-slate-950"
>
  <!-- Job list -->
  <div class="relative border-r text-sm">
    <div class="absolute inset-0 grid items-start overflow-auto">
      <div class="grid {showGroup ? 'grid-cols-3' : 'grid-cols-2'}">
        {#each showGroup ? ['Job', 'State', 'Group'] : ['Job', 'State'] as title}
          <div class="sticky top-0 bg-slate-950 px-2 py-1 text-center font-bold">{title}</div>
        {/each}
        {#each filteredJobs.slice().reverse() as job (job.id)}
          <button class="group contents text-left" on:click={() => ($selectedJobId = job.id)}>
            <div
              class="py-0.5 pl-2 pr-1 transition {job.id == $selectedJobId
                ? 'bg-slate-700'
                : 'group-hover:bg-slate-800'}"
            >
              {job.kind.kind == 'Compilation'
                ? 'Compilation'
                : job.kind.kind == 'Waiting'
                  ? '...'
                  : job.kind.data.analysis}
            </div>
            <div
              class="flex items-center justify-center px-1 py-0.5 transition {job.id ==
              $selectedJobId
                ? 'bg-slate-700'
                : 'group-hover:bg-slate-800'}"
              title={job.state}
            >
              <svelte:component
                this={Icon(job.state)}
                class="w-4 transition {icons[job.state][1]}"
              />
            </div>
            {#if showGroup}
              <div
                class="py-0.5 pl-2 pr-1 text-center transition {job.id == $selectedJobId
                  ? 'bg-slate-700'
                  : 'group-hover:bg-slate-800'}"
              >
                {#if job.group_name}
                  {job.group_name}
                {:else}
                  <span class="text-xs italic text-gray-400">None</span>
                {/if}
              </div>
            {/if}
          </button>
        {/each}
      </div>
    </div>
  </div>

  <!-- Job view -->
  {#if $selectedJob}
    <JobTabs selectedJob={$selectedJob} />
  {:else}
    <div class="bg-slate-900" />
  {/if}
</div>
