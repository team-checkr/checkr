<script lang="ts">
	import Ansi from '$lib/components/Ansi.svelte';
	import JobPane from '$lib/components/JobPane.svelte';
	import StatusBar from '$lib/components/StatusBar.svelte';
	import { compilationStatusStore } from '$lib/events';
	import { showStatus } from '$lib/jobs';

	import Fire from '~icons/heroicons/fire';
</script>

<div class="relative grid grid-rows-[1fr_auto]">
	<main class="grid">
		<slot />
	</main>

	{#if $showStatus}
		<div class="h-[35vh]">
			<JobPane />
		</div>
	{/if}

	{#if $compilationStatusStore && $compilationStatusStore.state == 'Failed'}
		<div class="absolute inset-0 mt-20 grid items-start justify-center">
			<div
				class="grid h-[60vh] w-[50em] grid-rows-[auto_1fr] overflow-hidden rounded-lg bg-slate-600 shadow-xl"
			>
				<div class="flex items-center justify-between bg-red-600 px-3 py-1">
					<h2 class="text-xl font-light italic">Compilation error</h2>
					<Fire class="text-lg text-red-200" />
				</div>
				<div class="relative h-full w-full">
					<div class="absolute inset-0 overflow-auto text-sm">
						{#if $compilationStatusStore.error_output}
							<pre class="p-3"><code><Ansi spans={$compilationStatusStore.error_output} /></code
								></pre>
						{/if}
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<StatusBar />
