<script lang="ts">
  import { type driver } from '$lib/api';
  import { createEventDispatcher } from 'svelte';

  import EllipsisHorizontal from '~icons/heroicons/ellipsis-horizontal';
  import ArrowPath from '~icons/heroicons/arrow-path';
  import Check from '~icons/heroicons/check';
  import NoSymbol from '~icons/heroicons/no-symbol';
  import Fire from '~icons/heroicons/fire';
  import ExclamationTriangle from '~icons/heroicons/exclamation-triangle';
  import Clock from '~icons/heroicons/clock';
  import Trash from '~icons/heroicons/Trash';

  export let state: driver.job.JobState;

  const dispatch = createEventDispatcher<{ click: void }>();

  const icons: Record<driver.job.JobState, [typeof EllipsisHorizontal, string, string]> = {
    // Queued: [EllipsisHorizontal, 'animate-pulse', ''],
    Queued: [EllipsisHorizontal, '', ''],
    Running: [ArrowPath, 'animate-spin', 'bg-slate-400'],
    Succeeded: [Check, '', 'bg-green-500'],
    Canceled: [NoSymbol, '', 'bg-slate-400'],
    Failed: [Fire, '', 'bg-red-500'],
    Warning: [ExclamationTriangle, '', 'bg-yellow-400'],
    Timeout: [Clock, '', 'bg-blue-400'],
    OutputLimitExceeded: [Trash, '', 'bg-orange-400'],
  };

  $: icon = icons[state][0];
  $: iconClass = icons[state][1];
  $: containerClass = icons[state][2];
</script>

<button
  class="grid h-full place-items-center p-2 transition-colors {containerClass}"
  on:click={() => dispatch('click')}
>
  <svelte:component this={icon} class="h-6 w-6 text-white transition-colors {iconClass}" />
</button>
