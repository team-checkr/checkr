<script lang="ts">
	import { type inspectify_api, type driver } from '$lib/api';
	import { groupProgramJobAssignedStore, jobsStore } from '$lib/events';

	import EllipsisHorizontal from '~icons/heroicons/ellipsis-horizontal';
	import ArrowPath from '~icons/heroicons/arrow-path';
	import Check from '~icons/heroicons/check';
	import NoSymbol from '~icons/heroicons/no-symbol';
	import Fire from '~icons/heroicons/fire';
	import ExclamationTriangle from '~icons/heroicons/exclamation-triangle';

	export let group: inspectify_api.checko.config.GroupConfig;
	export let program: inspectify_api.endpoints.Program;

	$: jobId = $groupProgramJobAssignedStore?.[group.name]?.[program.hash_str];
	$: job = $jobsStore[jobId];

	const icons: Record<driver.job.JobState, [typeof EllipsisHorizontal, string, string]> = {
		Queued: [EllipsisHorizontal, 'animate-pulse', ''],
		Running: [ArrowPath, 'animate-spin text-slate-400', 'bg-slate-400'],
		Succeeded: [Check, 'text-green-300', 'bg-green-500'],
		Canceled: [NoSymbol, 'text-slate-400', 'bg-slate-400'],
		Failed: [Fire, 'text-red-300', 'bg-red-500'],
		Warning: [ExclamationTriangle, 'text-yellow-300', 'bg-yellow-300']
	};
	const Icon = (state: driver.job.JobState) => icons[state][0];

	$: state = $job?.state ?? 'Queued';

	$: console.log(program.hash_str, $job);
</script>

<div class="grid h-full place-items-center p-2 transition {icons[state][2]}">
	<svelte:component this={Icon(state)} class="h-6 w-6 text-white transition {icons[state][1]}" />
</div>
