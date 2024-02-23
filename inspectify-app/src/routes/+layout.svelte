<script lang="ts">
  import { ce_shell } from '$lib/api';
  import { page } from '$app/stores';
  import '../app.pcss';

  import CommandLineIcon from '~icons/heroicons/command-line';
  // import QuestionMarkCircleIcon from '~icons/heroicons/question-mark-circle';
  import { showReference } from '$lib/jobs';

  const { ANALYSIS } = ce_shell;
</script>

<div class="grid h-screen grid-rows-[auto_1fr]">
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
    <!-- {
	  process.env.INSPECTIFY_VERSION && (
		<span class="text-xs place-self-end mb-2 text-slate-400 italic font-light">
		  {import.meta.env.INSPECTIFY_VERSION}
		</span>
	  )
	} -->

    <div class="ml-6 flex h-full text-base font-thin">
      {#each ANALYSIS as o}
        <a
          href="/env/{o}"
          class="flex items-center px-2 transition hover:bg-slate-800"
          class:active={$page.url.pathname == `/env/${o}`}>{o}</a
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
        bind:checked={$showReference}
      />
    </div>

    <!-- <a
      href="/guide"
      class="flex items-center space-x-1 p-2 text-sm font-semibold text-slate-300 transition hover:text-white"
    >
      <span>Guide</span>
      <QuestionMarkCircleIcon class="w-4" />
    </a> -->
  </nav>

  <slot />
</div>

<style>
  .active {
    @apply bg-slate-700;
  }
</style>
