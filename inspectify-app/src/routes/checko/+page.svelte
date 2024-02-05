<script lang="ts">
	import JobPane from '$lib/components/JobPane.svelte';
	import StatusBar from '$lib/components/StatusBar.svelte';

	let showStatus = true;
</script>

<JobPane showGroup />

<StatusBar bind:showStatus />
