<script lang="ts">
  import '../app.pcss';
</script>

<div class="grid h-screen grid-rows-[auto_1fr]">
  <slot />
</div>

<style>
  .active {
    @apply bg-slate-700;
  }
</style>
