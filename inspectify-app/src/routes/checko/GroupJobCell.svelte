<script lang="ts">
  import { type inspectify, type driver } from '$lib/api';
  import { groupProgramJobAssignedStore, jobsStore } from '$lib/events';
  import { selectedJobId, showStatus } from '$lib/jobs';

  import EllipsisHorizontal from '~icons/heroicons/ellipsis-horizontal';
  import ArrowPath from '~icons/heroicons/arrow-path';
  import Check from '~icons/heroicons/check';
  import NoSymbol from '~icons/heroicons/no-symbol';
  import Fire from '~icons/heroicons/fire';
  import ExclamationTriangle from '~icons/heroicons/exclamation-triangle';
  import Clock from '~icons/heroicons/clock';
  import Trash from '~icons/heroicons/Trash';

  export let group: inspectify.checko.config.GroupConfig;
  export let program: inspectify.endpoints.Program;

  $: jobId = $groupProgramJobAssignedStore?.[group.name]?.[program.hash_str];
  $: job = $jobsStore[jobId];

  const icons: Record<driver.job.JobState, [typeof EllipsisHorizontal, string, string]> = {
    Queued: [EllipsisHorizontal, 'animate-pulse', ''],
    Running: [ArrowPath, 'animate-spin', 'bg-slate-400'],
    Succeeded: [Check, '', 'bg-green-500'],
    Canceled: [NoSymbol, '', 'bg-slate-400'],
    Failed: [Fire, '', 'bg-red-500'],
    Warning: [ExclamationTriangle, '', 'bg-yellow-400'],
    Timeout: [Clock, '', 'bg-blue-400'],
    OutputLimitExceeded: [Trash, '', 'bg-orange-400'],
  };
  const Icon = (state: driver.job.JobState) => icons[state][0];

  $: state = $job?.state ?? 'Queued';
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
