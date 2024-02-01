<script lang="ts">
	import { driver } from '$lib/api';
	import { jobsStore } from '$lib/jobs';

	import ChevronDoubleUp from '~icons/heroicons/chevron-double-up';
	import Link from '~icons/heroicons/link';

	// TODO
	const version = '1.2.3';

	export let showStatus: boolean;

	let jobStates = Object.fromEntries(driver.job.JOB_STATE.map((s) => [s, 0])) as Record<
		driver.job.JobState,
		number
	>;

	$: {
		jobStates = Object.fromEntries(driver.job.JOB_STATE.map((s) => [s, 0])) as Record<
			driver.job.JobState,
			number
		>;

		for (let job of $jobsStore) {
			if (job.state in jobStates) {
				jobStates[job.state]++;
			}
		}
	}
</script>

<div class="flex items-center space-x-1 border-t bg-slate-900 text-sm">
	<button
		class="flex h-full items-center space-x-0.5 bg-slate-900 px-2 text-xs transition hover:bg-slate-400/10 active:bg-slate-400/5"
		on:click={() => (showStatus = !showStatus)}
	>
		<ChevronDoubleUp class="transition {showStatus ? 'rotate-0' : 'rotate-180'}" />
	</button>

	{#if jobStates['Queued'] === 0 && jobStates['Running'] === 0 && jobStates['Succeeded'] > 0 && jobStates['Failed'] === 0 && jobStates['Warning'] === 0}
		<p>No active jobs</p>
	{:else}
		<b>Jobs: </b>
		<i class="space-x-1">
			{#each Object.entries(jobStates) as [state, count] (state)}
				{#if count > 0}<span>{count} {state.toLowerCase()}</span>{/if}
			{/each}
		</i>
	{/if}

	<div class="flex-1"></div>

	<div class="text-xs text-slate-400">v{version}</div>
	<div class="place-self-end bg-green-600 p-1">
		<Link />
	</div>
</div>
