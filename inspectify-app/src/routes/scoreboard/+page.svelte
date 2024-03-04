<script lang="ts">
  import GroupJobCellView from '$lib/components/GroupJobCellView.svelte';
  import { publicData } from '$lib/public';
  import { onMount } from 'svelte';
  import { flip } from 'svelte/animate';

  let data = $publicData;
  $: numberOfPrograms = data.analysis.reduce((acc, analysis) => {
    return acc + analysis.programs.length;
  }, 0);

  const animationDuration = 500;

  onMount(() => {
    const interval = setInterval(() => {
      data = $publicData;
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
    {#each data.analysis as analysis}
      <div
        class="border px-3 py-2 text-center text-xl font-bold italic"
        style="grid-column: span {analysis.programs.length}"
      >
        {analysis.analysis}
      </div>
    {/each}
    {#each data.groups as group, index (group.name)}
      <div
        animate:flip={{ duration: animationDuration }}
        class="col-span-full grid"
        style="z-index: {data.groups.length - index};
               grid-template-columns: var(--name-width) repeat({numberOfPrograms},1fr);"
      >
        <div class="flex items-center justify-center border bg-slate-800 px-1 font-mono font-bold">
          {group.name}
        </div>
        {#each group.analysis_results as analysis}
          {#each analysis.results as res}
            <div class="-my-0.5 grid w-full">
              <GroupJobCellView state={res.state} />
            </div>
          {/each}
        {/each}
      </div>
    {/each}
  </div>
</div>
