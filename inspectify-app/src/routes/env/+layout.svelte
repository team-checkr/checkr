<script lang="ts">
	import JobPane from '$lib/components/JobPane.svelte';
	import StatusBar from '$lib/components/StatusBar.svelte';

	let showStatus = true;
</script>

<main class="grid">
	<slot />
</main>

{#if showStatus}
	<JobPane />
{/if}

<StatusBar bind:showStatus />
