<script lang="ts">
  import { ce_shell } from '$lib/api';
  import { page } from '$app/stores';

  import CommandLineIcon from '~icons/heroicons/command-line';
  import { showReference } from '$lib/jobs.svelte';
  interface Props {
    children?: import('svelte').Snippet;
  }

  let { children }: Props = $props();

  const { ANALYSIS } = ce_shell;
</script>

<nav class="flex items-center bg-slate-900 px-2 text-slate-200">
  <a href="/" class="flex items-center space-x-2 p-2 pr-0 text-2xl font-thin italic">
    <div class="relative">
      <CommandLineIcon
        class="absolute inset-0 left-0.5 top-0.5 w-6 animate-pulse text-teal-500/50"
      />
      <CommandLineIcon class="relative w-6" />
    </div>
    <span>Inspectify</span>
  </a>

  <div class="ml-6 flex h-full text-base font-thin">
    {#each ANALYSIS as o}
      <a
        href="/env/{o}"
        class="flex items-center px-2 transition hover:bg-slate-800 {$page.url.pathname ==
        `/env/${o}`
          ? 'bg-slate-700'
          : ''}">{o}</a
      >
    {/each}
  </div>

  <div class="flex-1"></div>

  <div
    class="flex select-none items-center space-x-0.5 p-2 text-xs font-semibold text-slate-300 transition hover:text-white"
  >
    <label for="show-reference" class="flex cursor-pointer items-center space-x-1 p-2"
      >Show reference</label
    >
    <input
      type="checkbox"
      name="show-reference"
      id="show-reference"
      bind:checked={showReference.show}
    />
  </div>
</nav>

{@render children?.()}
