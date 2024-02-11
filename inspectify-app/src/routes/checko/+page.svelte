<script lang="ts">
	import { ce_shell } from '$lib/api';
	import JobPane from '$lib/components/JobPane.svelte';
	import StatusBar from '$lib/components/StatusBar.svelte';
	import { groupsConfigStore, programsStore } from '$lib/events';
	import GroupJobCell from './GroupJobCell.svelte';

	$: includedAnalysis = ce_shell.ANALYSIS.filter((a) =>
		$programsStore.find((p) => p.input.analysis == a)
	);

	let showStatus = false;
</script>

<div class="grid {showStatus ? 'grid-cols-[1fr_1fr]' : ''}">
	{#if showStatus}
		<JobPane showGroup />
	{/if}
	<div
		class="grid self-start border-l"
		style="grid-template-columns: auto repeat({$programsStore.length}, 1fr);"
	>
		{#if $groupsConfigStore && $programsStore}
			<div />
			{#each includedAnalysis as analysis (analysis)}
				<div
					class="border px-3 py-2 text-center text-xl font-bold italic"
					style="grid-column: span {$programsStore.filter((p) => p.input.analysis == analysis)
						.length}"
				>
					{analysis}
				</div>
			{/each}
			{#each $groupsConfigStore.groups as group (group.name)}
				<div class="flex items-center border bg-slate-800 px-1 font-bold">
					{group.name}
				</div>
				{#each $programsStore as program (program.hash_str)}
					<GroupJobCell {group} {program} />
				{/each}
			{/each}
		{/if}
	</div>
</div>

<StatusBar bind:showStatus />
