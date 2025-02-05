<script lang="ts">
  import type { Writable } from 'svelte/store';

  import { GCL } from '$lib/api';
  import InputOption from './InputOption.svelte';

  interface Props {
    input: Writable<{ determinism: GCL.Determinism }>;
  }

  let { input }: Props = $props();
</script>

<InputOption title="Determinism">
  <div class="grid w-full grid-cols-2 gap-x-2 font-mono">
    {#each GCL.DETERMINISM as determinism}
      <div
        class="flex items-center justify-center rounded text-sm transition {$input.determinism ==
        determinism
          ? 'bg-slate-500'
          : 'bg-slate-800'}"
      >
        <label for="determinism-{determinism}" class="cursor-pointer px-2 py-1">
          {determinism}
        </label>
        <input
          class="hidden"
          type="radio"
          id="determinism-{determinism}"
          name="determinism"
          value={determinism}
          bind:group={$input.determinism}
        />
      </div>
    {/each}
  </div>
</InputOption>
