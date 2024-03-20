<script lang="ts">
  export let title: string = 'Options';
</script>

<h1 class="border-y p-2 pb-1 text-lg font-bold">{title}</h1>
<div class="grid grid-cols-[max-content_1fr] items-center gap-y-2 px-1 py-1">
  <slot />
</div>
