<script lang="ts">
	import { ce_shell } from '$lib/api';
	import { type Io } from '$lib/io';
	import { currentTab, selectedJobId, showStatus } from '$lib/jobs';

	export let io: Io<ce_shell.Analysis>;
	const { results } = io;
	$: outputState = $results.outputState;
	$: validation = $results.validation;
	$: latestJobId = $results.latestJobId;
</script>

<div
	class="col-span-full col-start-2 row-start-2 flex h-6 items-center justify-between text-sm transition {outputState ==
	'Current'
		? validation?.type == 'CorrectTerminated' || validation?.type == 'CorrectNonTerminated'
			? 'bg-green-600'
			: validation?.type == 'Failure'
				? 'bg-red-500'
				: validation?.type == 'Mismatch'
					? 'bg-orange-500'
					: 'bg-gray-500'
		: 'bg-gray-500'}"
>
	<div class="px-1.5 font-mono text-xs italic">
		{validation?.type == 'Failure'
			? validation.message
			: validation?.type == 'Mismatch'
				? validation.reason
				: ''}
	</div>
	<button
		class="h-full px-1.5 font-bold transition hover:bg-white/10"
		on:click={() => {
			$selectedJobId = latestJobId;
			$currentTab = 'Output';
			$showStatus = true;
		}}>See output</button
	>
</div>
