<script lang="ts">
  export let title: string;
</script>

<div class="px-2 py-0.5 text-sm font-bold">{title}</div>
<div class="w-full font-mono">
  <slot />
</div>
