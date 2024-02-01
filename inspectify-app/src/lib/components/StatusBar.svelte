<script lang="ts">
	import ChevronDoubleUp from '~icons/heroicons/chevron-double-up';
	import Link from '~icons/heroicons/link';

	// TODO
	const version = '1.2.3';

	export let showStatus: boolean;
</script>

<div class="flex items-center space-x-1 border-t bg-slate-900 text-sm">
	<button
		class="flex h-full items-center space-x-0.5 bg-slate-900 px-2 text-xs transition hover:bg-slate-400/10 active:bg-slate-400/5"
		on:click={() => (showStatus = !showStatus)}
	>
		<ChevronDoubleUp class="transition {showStatus ? 'rotate-0' : 'rotate-180'}" />
	</button>

	<div class="flex-1"></div>

	<div class="text-xs text-slate-400">v{version}</div>
	<div class="place-self-end bg-green-600 p-1">
		<Link />
	</div>
</div>
