<script lang="ts">
  import { ce_shell } from '$lib/api';
  import { type Io } from '$lib/io.svelte';
  import { currentTab, selectedJobId, showStatus } from '$lib/jobs.svelte';

  interface Props {
    io: Io<ce_shell.Analysis>;
  }

  let { io }: Props = $props();
  let outputState = $derived(io.results.outputState);
  let validation = $derived(io.results.validation);
</script>

<div
  class="col-span-full col-start-2 row-start-2 flex h-6 items-center justify-between text-sm transition {outputState ==
  'Current'
    ? validation?.type == 'Correct'
      ? 'bg-green-600'
      : validation?.type == 'Failure'
        ? 'bg-red-500'
        : validation?.type == 'Mismatch'
          ? 'bg-orange-500'
          : 'bg-gray-500'
    : 'bg-gray-500'}"
>
  <div class="line-clamp-1 px-1.5 font-mono text-xs italic">
    {validation?.type == 'Failure'
      ? validation.message
      : validation?.type == 'Mismatch'
        ? validation.reason
        : ''}
  </div>
  <!-- TODO: This should display output in the output not the job pane -->
  <button
    class="h-full shrink-0 px-1.5 font-bold transition hover:bg-white/10"
    onclick={() => {
      selectedJobId.jobId = io.results.job ? io.results.job.id : null;
      currentTab.current = 'Output';
      showStatus.show = true;
    }}>See output</button
  >
</div>
