<script lang="ts">
	import { browser } from '$app/environment';
	import { api, ce_sign, type gcl } from '$lib/api';
	import Editor from '$lib/components/Editor.svelte';
	import Network from '$lib/components/Network.svelte';
	import { useIo } from '$lib/io';

	const io = useIo('Sign');
	const input = io.input;
	const output = io.output;

	let commands = '';
	let externallySet = '';
	let determinism: gcl.pg.Determinism = { Case: 'Deterministic' };

	$: if (commands && commands != externallySet) {
		input.set({
			commands,
			determinism,
			assignment: {
				variables: {},
				arrays: {}
			}
		});
	}

	const regenerate = async () => {
		const newInput = await io.generate();
		externallySet = newInput.commands;
		commands = newInput.commands;
	};

	let dot = '';
	$: if (browser && commands) {
		api.gclDot({ determinism, commands }).then((d) => {
			dot = d.dot;
			console.log({ dot });
		});
	}

	$: vars = $output
		? Object.values($output.nodes)
				.flatMap((assignment) =>
					assignment.flatMap((mem) => [...Object.keys(mem.arrays), ...Object.keys(mem.variables)])
				)
				.filter((v, i, a) => a.indexOf(v) == i)
		: [];

	const fmtSignOrSigns = (sign: ce_sign.semantics.Sign | ce_sign.semantics.Signs): string =>
		Array.isArray(sign)
			? sign.map(fmtSignOrSigns).join(' | ')
			: { Positive: '+', Zero: '0', Negative: '-' }[sign.Case];
</script>

<div class="grid grid-cols-[45ch_1fr_1fr] grid-rows-[1fr_auto]">
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
			<Network {dot} />
		</div>
	</div>
	<div class="relative">
		<div class="absolute inset-0 overflow-auto">
			{#if $output}
				<div
					class="grid w-full grid-flow-dense [&_*]:border-t"
					style="grid-template-columns: repeat({vars.length + 1}, auto);"
				>
					<div class="border-none"></div>
					{#each vars as v}
						<div class="border-none text-center">{v}</div>
					{/each}
					{#each Object.entries($output.nodes) as [node, mems]}
						{#each mems as mem, idx}
							{#if idx == 0}
								<h2 class="px-2" style="grid-row: span {mems.length} / span {mems.length};">
									{node}
								</h2>
							{/if}
							{#each vars as v}
								<div class="px-2 py-0.5 font-mono text-sm">
									{v}: {fmtSignOrSigns(v in mem.arrays ? mem.arrays[v] : mem.variables[v])}
								</div>
							{/each}
						{/each}
					{/each}
				</div>
			{/if}
		</div>
	</div>
</div>
