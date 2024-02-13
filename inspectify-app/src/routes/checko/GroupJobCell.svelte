<script lang="ts">
  import { type inspectify_api, type driver } from '$lib/api';
  import { groupProgramJobAssignedStore, jobsStore } from '$lib/events';

  import EllipsisHorizontal from '~icons/heroicons/ellipsis-horizontal';
  import ArrowPath from '~icons/heroicons/arrow-path';
  import Check from '~icons/heroicons/check';
  import NoSymbol from '~icons/heroicons/no-symbol';
  import Fire from '~icons/heroicons/fire';
  import ExclamationTriangle from '~icons/heroicons/exclamation-triangle';
  import { selectedJobId, showStatus } from '$lib/jobs';

  export let group: inspectify_api.checko.config.GroupConfig;
  export let program: inspectify_api.endpoints.Program;

  $: jobId = $groupProgramJobAssignedStore?.[group.name]?.[program.hash_str];
  $: job = $jobsStore[jobId];

  const icons: Record<driver.job.JobState, [typeof EllipsisHorizontal, string, string]> = {
    Queued: [EllipsisHorizontal, 'animate-pulse', ''],
    Running: [ArrowPath, 'animate-spin', 'bg-slate-400'],
    Succeeded: [Check, '', 'bg-green-500'],
    Canceled: [NoSymbol, '', 'bg-slate-400'],
    Failed: [Fire, '', 'bg-red-500'],
    Warning: [ExclamationTriangle, '', 'bg-yellow-400'],
  };
  const Icon = (state: driver.job.JobState) => icons[state][0];

  $: state = $job?.state ?? 'Queued';

  $: console.log(program.hash_str, $job);
</script>

<button
  class="grid h-full place-items-center p-2 transition {icons[state][2]}"
  on:click={() => {
    $selectedJobId = jobId;
    $showStatus = true;
  }}
>
  <svelte:component this={Icon(state)} class="h-6 w-6 text-white transition {icons[state][1]}" />
</button>
