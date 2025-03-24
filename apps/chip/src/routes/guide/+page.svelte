<script lang="ts">
  import Guide from '$lib/components/Guide.svelte';
  import Nav from '$lib/components/Nav.svelte';
  import Icon from '~icons/heroicons/book-open';
</script>

<svelte:head>
  <title>Chip – Guide</title>
  <meta name="description" content="Guide" />
</svelte:head>

<Nav title="Guide" {Icon}></Nav>

<div class="py-10">
  <Guide />
</div>
