<script lang="ts">
	import Network from '$lib/components/Network.svelte';
	import StandardInput from '$lib/components/StandardInput.svelte';
	import ValidationIndicator from '$lib/components/ValidationIndicator.svelte';
	import { useIo } from '$lib/io';

	const io = useIo(
		'Graph',
		{
			commands: 'skip',
			determinism: { Case: 'Deterministic' }
		},
		{
			dot: 'digraph G {}'
		}
	);
	const output = io.output;
</script>

<div class="grid grid-cols-[45ch_1fr] grid-rows-[1fr_auto]">
	<StandardInput analysis="Graph" code="commands" {io} />
	<div class="relative">
		<div class="absolute inset-0 grid overflow-auto">
			<Network dot={$output.dot || ''} />
		</div>
	</div>
	<ValidationIndicator {io} />
</div>
