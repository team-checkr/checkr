<script lang="ts" generics="A extends ce_shell.Analysis">
	import { ce_shell } from '$lib/api';
	import Editor from '$lib/components/Editor.svelte';
	import type { Io, Input } from '$lib/io';

	import ClipboardDocumentList from '~icons/heroicons/clipboard-document-list';

	export let analysis: A;
	export let io: Io<A>;
	type StringFields = {
		[K in keyof Input<A>]: Input<A>[K] extends string ? K : never;
	}[keyof Input<A>];
	export let code: StringFields | undefined = void 0;

	const input = io.input;

	const regenerate = async () => {
		$input = await io.generate();
	};
	const copyInput = () => {
		navigator.clipboard.writeText(JSON.stringify($input));
	};
</script>

<div class="row-span-full grid grid-rows-[auto_1fr]">
	<div class="items-ce flex border-r bg-slate-950">
		<button on:click={regenerate} class="px-1.5 py-1 transition hover:bg-slate-800">Generate</button
		>
		<div class="flex-1" />
		<button on:click={copyInput} class="px-1.5 py-1 transition hover:bg-slate-800"
			><ClipboardDocumentList /></button
		>
	</div>
	<div class="relative row-span-2 border-r">
		<div class="absolute inset-0 grid overflow-auto">
			{#if code}
				<Editor bind:value={$input[code]} />
			{/if}
		</div>
	</div>
	{#if $$slots.default}
		<div class="border-r">
			<slot />
		</div>
	{/if}
</div>
