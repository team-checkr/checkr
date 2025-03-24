<script lang="ts">
  import { theme } from '$lib/theme';

  import Sun from '~icons/heroicons/sun';
  import Moon from '~icons/heroicons/moon';
  import QuestionMarkCircle from '~icons/heroicons/question-mark-circle';
  import type { Component } from 'svelte';
  import type { SvelteHTMLElements } from 'svelte/elements';
  import Guide from './Guide.svelte';

  interface Props {
    title: string;
    Icon: Component<SvelteHTMLElements['svg']>;
  }

  let { title, Icon }: Props = $props();

  let showGuide = $state(false);

  const toggleGuide = (e: MouseEvent) => {
    e.preventDefault();
    showGuide = !showGuide;
  };

  let darkTheme = $state($theme == 'dark');
  $effect(() => {
    if (darkTheme) {
      $theme = 'dark';
    } else {
      $theme = 'light';
    }
  });

  $effect(() => {
    const listener = (e: KeyboardEvent) => {
      if (showGuide && e.key == 'Escape') {
        showGuide = false;
      }
    };
    window.addEventListener('keydown', listener);

    return () => window.removeEventListener('keydown', listener);
  });
</script>

<nav class="flex items-center space-x-2 bg-slate-900 px-2 text-slate-200">
  <a href="/" class="flex items-center space-x-2 p-2 pr-0 text-2xl font-thin italic">
    <div class="relative">
      <Icon class="absolute inset-0 left-0.5 top-0.5 w-6 animate-pulse text-teal-500/50" />
      <Icon class="relative w-6" />
    </div>
    <span>{title}</span>
  </a>
  <div class="flex-1"></div>
  <div>
    <label for="theme" class="flex cursor-pointer select-none items-center space-x-1">
      <span>Switch theme</span>
      <div class="relative h-5 w-5">
        <Sun
          class="absolute inset-0 transition {$theme == 'light' ? 'opacity-100' : 'opacity-0'}"
        />
        <Moon
          class="absolute inset-0 transition {$theme == 'dark' ? 'opacity-100' : 'opacity-0'}"
        />
      </div>
    </label>
    <input class="hidden" type="checkbox" name="theme" id="theme" bind:checked={darkTheme} />
  </div>
  <a href="/guide" class="flex items-center space-x-1 p-2" onclick={toggleGuide}>
    <span>Guide</span>
    <QuestionMarkCircle />
  </a>
</nav>

{#if showGuide}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="z-100 fixed inset-0 grid place-items-center" onclick={() => (showGuide = false)}>
    <div
      class="relative max-h-[80vh] overflow-auto rounded-xl bg-slate-800 shadow-2xl"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="px-10 py-5">
        <Guide />
      </div>
    </div>
  </div>
{/if}
