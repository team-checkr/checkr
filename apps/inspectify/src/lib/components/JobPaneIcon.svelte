<script lang="ts">
  import { type driver } from '$lib/api';

  import EllipsisHorizontal from '~icons/heroicons/ellipsis-horizontal';
  import ArrowPath from '~icons/heroicons/arrow-path';
  import Check from '~icons/heroicons/check';
  import NoSymbol from '~icons/heroicons/no-symbol';
  import Fire from '~icons/heroicons/fire';
  import ExclamationTriangle from '~icons/heroicons/exclamation-triangle';
  import Clock from '~icons/heroicons/clock';
  import Trash from '~icons/heroicons/Trash';

  interface Props {
    jobState: driver.job.JobState;
  }

  const { jobState }: Props = $props();

  const icons: Record<driver.job.JobState, [typeof EllipsisHorizontal, string]> = {
    Queued: [EllipsisHorizontal, 'animate-pulse'],
    Running: [ArrowPath, 'animate-spin text-slate-400'],
    Succeeded: [Check, 'text-green-300'],
    Canceled: [NoSymbol, 'text-slate-400'],
    Failed: [Fire, 'text-red-300'],
    Warning: [ExclamationTriangle, 'text-yellow-300'],
    Timeout: [Clock, 'text-blue-300'],
    OutputLimitExceeded: [Trash, 'text-orange-300'],
  };

  const [Icon, classes] = $derived(icons[jobState]);
</script>

<Icon class="w-4 transition {classes}" />
