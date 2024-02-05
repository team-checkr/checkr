<script lang="ts">
	import { driver } from '$lib/api';
	import { jobsListStore, jobsStore } from '$lib/events';

	import EllipsisHorizontal from '~icons/heroicons/ellipsis-horizontal';
	import ArrowPath from '~icons/heroicons/arrow-path';
	import Check from '~icons/heroicons/check';
	import NoSymbol from '~icons/heroicons/no-symbol';
	import Fire from '~icons/heroicons/fire';
	import ExclamationTriangle from '~icons/heroicons/exclamation-triangle';
	import Ansi from '$lib/components/Ansi.svelte';
	import JsonView from './JSONView.svelte';
	import { derived } from 'svelte/store';

	export let showGroup = false;

	const icons: Record<driver.job.JobState, [typeof EllipsisHorizontal, string]> = {
		Queued: [EllipsisHorizontal, 'animate-pulse'],
		Running: [ArrowPath, 'animate-spin text-slate-400'],
		Succeeded: [Check, 'text-green-300'],
		Canceled: [NoSymbol, 'text-slate-400'],
		Failed: [Fire, 'text-red-300'],
		Warning: [ExclamationTriangle, 'text-yellow-300']
	};

	const Icon = (state: driver.job.JobState) => icons[state][0];

	$: jobs = derived(
		$jobsListStore.map((id) => jobsStore[id]),
		(jobs) => jobs
	);
	$: filteredJobs = $jobs.filter((j) => j.state != 'Canceled');
	// $: filteredJobs = $jobs.filter((j) => j.state != 'Canceled');
	let selectedJobId: null | driver.job.JobId = null;
	$: selectedJob = typeof selectedJobId == 'number' ? jobsStore[selectedJobId] : null;
	// $: if (selectedJobId == null || !filteredJobs.includes(selectedJob)) {
	// 	selectedJobId = $jobsStore.length > 0 ? $jobsStore[$jobsStore.length - 1].id : null;
	// }
	type Output =
		| {
				kind: 'parsed';
				parsed: any;
		  }
		| {
				kind: 'parse error';
				raw: string;
		  };
	let output: Output | null = null;
	$: if ($selectedJob) {
		try {
			output = {
				kind: 'parsed',
				parsed: JSON.parse($selectedJob.stdout)
			};
		} catch (e) {
			output = {
				kind: 'parse error',
				raw: $selectedJob.stdout
			};
		}
	}

	const tabs = ['Output', 'Input JSON', 'Output JSON', 'Reference Output', 'Validation'] as const;
	type Tab = (typeof tabs)[number];
	let currentTab: Tab = tabs[0];
	$: if ($selectedJob?.kind.kind == 'Compilation') {
		currentTab = 'Output';
	}
	$: isDisabled = (tab: Tab) =>
		$selectedJob ? tab != 'Output' && $selectedJob.kind.kind == 'Compilation' : true;
</script>

<div
	class="z-10 grid h-full {showGroup
		? 'grid-cols-[25ch_1fr]'
		: 'grid-cols-[20ch_1fr]'} border-t bg-slate-950"
>
	<!-- Job list -->
	<div class="relative border-r text-sm">
		<div class="absolute inset-0 grid items-start overflow-auto">
			<div class="grid {showGroup ? 'grid-cols-3' : 'grid-cols-2'}">
				{#each showGroup ? ['Job', 'State', 'Group'] : ['Job', 'State'] as title}
					<div class="sticky top-0 bg-slate-950 px-2 py-1 text-center font-bold">{title}</div>
				{/each}
				{#each filteredJobs.slice().reverse() as job (job.id)}
					<button class="group contents text-left" on:click={() => (selectedJobId = job.id)}>
						<div
							class="py-0.5 pl-2 pr-1 transition {job.id == selectedJobId
								? 'bg-slate-700'
								: 'group-hover:bg-slate-800'}"
						>
							{job.kind.kind == 'Compilation'
								? 'Compilation'
								: job.kind.kind == 'Waiting'
									? '...'
									: job.kind.data[0]}
						</div>
						<div
							class="flex items-center justify-center px-1 py-0.5 transition {job.id ==
							selectedJobId
								? 'bg-slate-700'
								: 'group-hover:bg-slate-800'}"
							title={job.state}
						>
							<svelte:component
								this={Icon(job.state)}
								class="w-4 transition {icons[job.state][1]}"
							/>
						</div>
						{#if showGroup}
							<div
								class="py-0.5 pl-2 pr-1 text-center transition {job.id == selectedJobId
									? 'bg-slate-700'
									: 'group-hover:bg-slate-800'}"
							>
								{#if job.group_name}
									{job.group_name}
								{:else}
									<span class="text-xs italic text-gray-400">None</span>
								{/if}
							</div>
						{/if}
					</button>
				{/each}
			</div>
		</div>
	</div>

	<!-- Job view -->
	{#if $selectedJob}
		<div class="grid grid-rows-[auto_1fr]">
			<div class="flex text-sm">
				{#each tabs as tab}
					<button
						class="flex-1 px-2 py-1 transition disabled:opacity-50 {tab == currentTab ||
						isDisabled(tab)
							? 'bg-slate-700'
							: 'hover:bg-slate-800'}"
						on:click={() => (currentTab = tab)}
						disabled={isDisabled(tab)}
					>
						{tab}
					</button>
				{/each}
			</div>
			<div class="relative self-stretch bg-slate-900 text-xs">
				<div class="absolute inset-0 overflow-auto">
					{#if currentTab == 'Output'}
						<pre class="p-3 [overflow-anchor:none]"><code><Ansi spans={$selectedJob.spans} /></code
							></pre>
						<div class="[overflow-anchor:auto]" />
					{:else if currentTab == 'Input JSON' && $selectedJob.kind.kind == 'Analysis'}
						<JsonView json={$selectedJob.kind.data[1].json} />
						<div class="[overflow-anchor:auto]" />
					{:else if currentTab == 'Output JSON'}
						{#if output}
							{#if output.kind == 'parsed'}
								<JsonView json={output.parsed} />
							{:else if output.kind == 'parse error'}
								<div class="p-2">
									<div class="italic text-red-500">Failed to parse JSON</div>
									{#if output.raw.length > 0}
										<pre class="p-3 [overflow-anchor:none]"><code>{output.raw}</code></pre>
									{:else}
										<pre class="p-3 italic text-gray-400 [overflow-anchor:none]"><code
												>&lt;stdout was empty&gt;</code
											></pre>
									{/if}
								</div>
							{/if}
						{/if}
						<!-- <JsonView json={JSON.parse(selectedJob.stdout)} /> -->
						<div class="[overflow-anchor:auto]" />
					{:else if currentTab == 'Reference Output'}
						<JsonView json={$selectedJob.analysis_data?.reference_output.json} />
						<div class="[overflow-anchor:auto]" />
					{:else if currentTab == 'Validation'}
						<JsonView json={$selectedJob.analysis_data?.validation} />
						<div class="[overflow-anchor:auto]" />
					{/if}
				</div>
			</div>
		</div>
	{:else}
		<div class="bg-slate-900" />
	{/if}
</div>
