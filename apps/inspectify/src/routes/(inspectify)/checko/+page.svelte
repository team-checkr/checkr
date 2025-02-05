<!-- @migration-task Error while migrating Svelte code: can't migrate `$: includedAnalysis = ce_shell.ANALYSIS.filter((a) =>
    $programsStore.find((p) => p.input.analysis == a),
  );` to `$derived` because there's a variable named derived.
     Rename the variable and try again or migrate by hand. -->
<script lang="ts">
  import { ce_shell, driver, type inspectify } from '$lib/api';
  import JobPane from '$lib/components/JobPane.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import {
    groupProgramJobAssignedStore,
    groupsConfigStore,
    jobsStore,
    programsStore,
  } from '$lib/events';
  import { showStatus } from '$lib/jobs';
  import { derived, readonly, writable } from 'svelte/store';
  import GroupJobCell from './GroupJobCell.svelte';

  $: includedAnalysis = ce_shell.ANALYSIS.filter((a) =>
    $programsStore.find((p) => p.input.analysis == a),
  );

  // $: computeGroupState = (group: inspectify.checko.config.GroupConfig) => {
  //   const states = $programsStore.map((program) => {
  //     const jobId = $groupProgramJobAssignedStore?.[group.name]?.[program.hash_str];
  //     const job = $jobsStore[jobId];
  //     if (!job) return writable('Queued' as const);
  //     return derived([job], ([job]) => {
  //       const validation = job?.analysis_data?.validation?.type;
  //       return validation == 'Mismatch' ? 'Warning' : job?.state ?? 'Queued';
  //     });
  //   });

  //   return derived(
  //     states,
  //     (states) =>
  //       states.filter((s) => s == 'Succeeded').length - states.filter((s) => s == 'Failed').length,
  //   );
  // };

  // $: scores = derived(
  //   $groupsConfigStore?.groups.map((group) => {
  //     return computeGroupState(group);
  //   }) || [],
  //   (xs) => xs,
  // );

  // $: sortedGroups =
  //   $groupsConfigStore?.groups.slice().sort((a, b) => {
  //     const aIndex = $groupsConfigStore?.groups.indexOf(a) ?? -1;
  //     const bIndex = $groupsConfigStore?.groups.indexOf(b) ?? -1;

  //     if (aIndex == -1 || bIndex == -1) return 0;

  //     return $scores[bIndex] - $scores[aIndex];
  //   }) || [];

  $: sortedGroups = $groupsConfigStore?.groups || [];
</script>

<div class="grid {showStatus ? 'grid-cols-[auto_1fr]' : ''}">
  {#if showStatus}
    <JobPane showGroup />
  {/if}
  <div class="w-full overflow-auto">
    <div
      class="grid self-start border-l"
      style="grid-template-columns: auto repeat({$programsStore.length}, 1fr);"
    >
      {#if $groupsConfigStore && $programsStore}
        <div></div>
        {#each includedAnalysis as analysis (analysis)}
          <div
            class="border px-3 py-2 text-center text-xl font-bold italic"
            style="grid-column: span {$programsStore.filter((p) => p.input.analysis == analysis)
              .length}"
          >
            {analysis}
          </div>
        {/each}
        {#each sortedGroups as group (group.name)}
          <div class="flex items-center border bg-slate-800 px-1 font-bold">
            {group.name}
          </div>
          {#each $programsStore as program (program.hash_str)}
            <GroupJobCell {group} {program} />
          {/each}
        {/each}
      {/if}
    </div>
  </div>
</div>

<StatusBar />
