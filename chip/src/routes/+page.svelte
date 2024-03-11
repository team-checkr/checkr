<script lang="ts">
	import { writable } from 'svelte/store';
	import { browser } from '$app/environment';
	import Editor from '$lib/components/Editor.svelte';
	import type { MarkerData, ParseResult } from 'chip-wasm';

	let program = `{a=A}
if a > 0 -> a := a + 1
[] a = 0 -> a := 1
[] a < 0 -> a := a {a>A}
fi
{a>A} ;

{n >= 0}
i := 0 ; sum := 0 ;
do[{i <= n & sum = i * (i-1)/2}]
    i < n ->
        sum := sum + i ;
        i := i + 1
od
{sum = n * (n-1)/2}`;

	let result: ParseResult = {
		parse_error: false,
		assertions: [],
		markers: []
	};
	let verifications = writable<MarkerData[]>([]);

	let parseError = writable(false);

	const STATES = ['idle', 'verifying', 'verified', 'error'];
	type State = (typeof STATES)[number];
	let state = writable<State>('idle');

	$: {
		const run = async () => {
			parseError.set(false);
			const { default: init, parse } = await import('chip-wasm');
			await init();
			const res = parse(program);
			if (res.parse_error) parseError.set(true);
			result = res;
		};
		run().catch(console.error);
	}
	$: {
		if (browser) {
			const run = async () => {
				const z3 = await import('$lib/z3');
				verifications.set([]);
				state.set('verifying');
				let errors = false;
				for (const t of result.assertions) {
					const res = await z3.run(t.smt);
					const valid = res[res.length - 1].trim() === 'unsat';

					if (!valid) {
						errors = true;
						verifications.update((res) => [
							...res,
							{
								severity: 'Error',
								tags: [],
								message: `Verification failed`,
								span: t.span,
								relatedInformation: []
							}
						]);
					}
				}
				if (errors) {
					state.set('error');
				} else {
					state.set('verified');
				}
			};
			run().catch(console.error);
		}
	}
</script>

<div class="relative grid grid-rows-[2fr_auto_auto] overflow-hidden">
	<Editor bind:value={program} markers={[...result.markers, ...$verifications]} />
	<div
		class="h-8 transition duration-500 {{
			idle: 'bg-gray-500',
			verifying: 'bg-yellow-500',
			verified: 'bg-green-500',
			error: 'bg-red-500'
		}[$parseError ? 'error' : $state]}"
	></div>
	<!-- <div>
		{#each result.assertions as triple}
			<pre class="p-4">{triple.smt}</pre>
		{/each}
	</div> -->
</div>
