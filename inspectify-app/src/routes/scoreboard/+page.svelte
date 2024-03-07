<script lang="ts">
  import { publicDataAnalysisStore, publicDataGroupsStore, lastFinishedStore } from '$lib/public';
  import { onMount } from 'svelte';
  import { flip } from 'svelte/animate';
  import GroupRow from './GroupRow.svelte';

  import CommandLineIcon from '~icons/heroicons/chart-bar-square';

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

<nav class="flex items-center bg-slate-900 px-2 text-sm text-slate-200">
  <a href="/" class="flex items-center space-x-2 p-2 pr-0 text-2xl font-thin italic">
    <div class="relative">
      <CommandLineIcon
        class="absolute inset-0 left-0.5 top-0.5 w-6 animate-pulse text-teal-500/50"
      />
      <CommandLineIcon class="relative w-6" />
    </div>
    <span>Checko</span>
  </a>

  <div class="flex-1" />
  <div class="flex space-x-1 py-1">
    <span class="italic text-slate-400">Last update:</span>
    <span class="font-mono"
      >{$lastFinishedStore &&
        new Intl.DateTimeFormat('en-GB', {
          hour: 'numeric',
          minute: 'numeric',
          second: 'numeric',
          day: 'numeric',
          month: 'numeric',
          year: 'numeric',
        }).format($lastFinishedStore)}</span
    >
  </div>
</nav>

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
