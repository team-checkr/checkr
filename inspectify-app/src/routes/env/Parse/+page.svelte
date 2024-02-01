<script lang="ts">
	import { browser } from '$app/environment';
	import { api, ce_sign, type gcl } from '$lib/api';
	import Editor from '$lib/components/Editor.svelte';
	import Network from '$lib/components/Network.svelte';
	import { useIo } from '$lib/io';

	const io = useIo('Parse');
	const input = io.input;
	const output = io.output;

	let commands = '';
	let externallySet = '';

	$: if (commands && commands != externallySet) {
		input.set({ commands });
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
			<pre class="p-2"><code
					>{#if $output}{$output.pretty}{/if}</code
				></pre>
		</div>
	</div>
</div>
