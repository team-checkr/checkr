<script lang="ts">
	import { type gcl } from '$lib/api';
	import Editor from '$lib/components/Editor.svelte';
	import Network from '$lib/components/Network.svelte';
	import { useIo } from '$lib/io';

	const color = 'idk';

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
	const input = io.input;
	const output = io.output;

	let commands = '';
	let externallySet = '';
	let determinism: gcl.pg.Determinism = { Case: 'Deterministic' };

	$: if (commands && commands != externallySet) {
		input.set({
			commands,
			determinism
		});
	}

	const regenerate = async () => {
		const newInput = await io.generate();
		externallySet = newInput.commands;
		commands = newInput.commands;
	};
</script>

<div class="grid grid-cols-[45ch_1fr] grid-rows-[1fr_auto]">
	<div class="grid grid-rows-[auto_1fr]">
		<div>
			<button on:click={regenerate}>Generate</button>
		</div>
		<div class="relative row-span-2 border-r">
			<div class="absolute inset-0 grid overflow-auto">
				<Editor bind:value={commands} />
			</div>
		</div>
	</div>
	<div class="relative">
		<div class="absolute inset-0 grid overflow-auto">
			<Network dot={$output.dot || ''} />
		</div>
	</div>
	<div class="h-4 bg-green-500 transition {color}"></div>
</div>
