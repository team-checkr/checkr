<script lang="ts">
  import InputOption from './InputOption.svelte';

  let {
    level = $bindable(),
    LEVELS,
  }: {
    level: number;
    LEVELS: {
      n: number;
      name: string;
    }[];
  } = $props();

  const currentName = $derived(LEVELS.find((l) => l.n === level)?.name ?? '');
</script>

<InputOption title="Level">
  <div class="flex flex-col gap-y-1">
    <div class="grid w-full grid-cols-7 gap-x-1 font-mono">
      {#each LEVELS as { n }}
        <button
          onclick={() => (level = n)}
          class="rounded py-1 text-center text-xs transition {n <= level
            ? 'bg-slate-500 text-white'
            : 'bg-slate-800 text-slate-500'}"
        >
          {n}
        </button>
      {/each}
    </div>
    <div class="text-xs text-slate-400">{currentName}</div>
  </div>
</InputOption>
