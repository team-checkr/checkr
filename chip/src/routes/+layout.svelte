<script lang="ts">
  import { theme } from '$lib/theme';
  import '../app.pcss';

  import Icon from '~icons/heroicons/check-badge';
  import Sun from '~icons/heroicons/sun';
  import Moon from '~icons/heroicons/moon';
  import QuestionMarkCircle from '~icons/heroicons/question-mark-circle';

  let darkTheme = $theme == 'dark';
  $: {
    if (darkTheme) {
      $theme = 'dark';
    } else {
      $theme = 'light';
    }
  }
</script>

<div class="grid h-screen grid-rows-[auto_1fr]">
  <nav class="flex items-center space-x-2 bg-slate-900 px-2 text-slate-200">
    <a href="/" class="flex items-center space-x-2 p-2 pr-0 text-2xl font-thin italic">
      <div class="relative">
        <Icon class="absolute inset-0 left-0.5 top-0.5 w-6 animate-pulse text-teal-500/50" />
        <Icon class="relative w-6" />
      </div>
      <span>Chip</span>
    </a>
    <div class="flex-1" />
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
    <a href="/guide" class="flex items-center space-x-1 p-2">
      <span>Guide</span>
      <QuestionMarkCircle />
    </a>
  </nav>

  <slot />
</div>
