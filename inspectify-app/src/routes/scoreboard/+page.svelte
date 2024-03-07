<script lang="ts">
  import { publicDataAnalysisStore, publicDataGroupsStore } from '$lib/public';
  import { onMount } from 'svelte';
  import { flip } from 'svelte/animate';
  import GroupRow from './GroupRow.svelte';

  let analysisStore = $publicDataAnalysisStore;
  let groupsStore = $publicDataGroupsStore;
  $: numberOfPrograms = $publicDataAnalysisStore.reduce((acc, analysis) => {
    return acc + analysis.programs.length;
  }, 0);

  const animationDuration = 500;

  onMount(() => {
    const interval = setInterval(() => {
      analysisStore = $publicDataAnalysisStore;
      groupsStore = $publicDataGroupsStore;
    }, animationDuration * 2);
    return () => clearInterval(interval);
  });
</script>

<div class="w-full overflow-auto" style="--name-width: 5rem">
  <div
    class="grid self-start border-l"
    style="grid-template-columns: var(--name-width) repeat({numberOfPrograms}, 1fr);"
  >
    <div />
    {#each analysisStore as analysis (analysis)}
      <div
        class="border px-3 py-2 text-center text-xl font-bold italic"
        style="grid-column: span {analysis.programs.length}"
      >
        {analysis.analysis}
      </div>
    {/each}
    {#each groupsStore as group, index (group.name)}
      <div
        animate:flip={{ duration: animationDuration }}
        class="col-span-full grid transform-gpu will-change-transform"
        style="z-index: {groupsStore.length - index};
               grid-template-columns: var(--name-width) 1fr;"
      >
        <GroupRow {group} {numberOfPrograms} />
      </div>
    {/each}
  </div>
</div>
