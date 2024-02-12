<script lang="ts">
	import StandardInput from '$lib/components/StandardInput.svelte';
	import ValidationIndicator from '$lib/components/ValidationIndicator.svelte';
	import { useIo } from '$lib/io';

	const io = useIo('Calc', { expression: '1 + 2' }, { result: '...', error: null });
	const { results } = io;
	$: output = $results.output;
	$: referenceOutput = $results.referenceOutput;
</script>

<div class="grid grid-cols-[40vw_1fr_1fr] grid-rows-[1fr_auto]">
	<StandardInput analysis="Calc" code="expression" {io} />
	<div class="col-span-1 row-span-1 border-r">
		<h1 class="border-t bg-slate-900 p-2 text-2xl font-light italic">Output</h1>
		{#if output.result}
			<h2 class="p-2 text-lg font-bold italic text-green-400">Result</h2>
			<pre class="rounded-md px-2 text-base">{output.result}</pre>
		{:else if output.error}
			<h2 class="p-2 text-lg font-bold italic text-orange-400">Evaluation error</h2>
			<pre class="rounded-md px-2 text-base">{output.error}</pre>
		{/if}
	</div>
	<div class="col-span-1 row-span-1">
		<h1 class="border-t bg-slate-900 p-2 text-2xl font-light italic">Reference</h1>
		{#if referenceOutput.result}
			<h2 class="p-2 text-lg font-bold italic text-green-400">Result</h2>
			<pre class="rounded-md px-2 text-base">{referenceOutput.result}</pre>
		{:else if referenceOutput.error}
			<h2 class="p-2 text-lg font-bold italic text-orange-400">Evaluation error</h2>
			<pre class="rounded-md px-2 text-base">{referenceOutput.error}</pre>
		{/if}
	</div>
	<ValidationIndicator {io} />
</div>
