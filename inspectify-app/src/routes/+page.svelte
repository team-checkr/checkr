<script lang="ts">
	import { PUBLIC_API_BASE } from '$env/static/public';
	import { api, type Analysis, ANALYSIS, setGlobalApiBase } from '$lib/api';
	import Editor from '$lib/components/Editor.svelte';
	import { onMount } from 'svelte';

	import CommandLineIcon from '~icons/heroicons/command-line';
	import PlayCircleIcon from '~icons/heroicons/play-circle';
	import QuestionMarkCircleIcon from '~icons/heroicons/question-mark-circle';

	let analysis: Analysis = ANALYSIS[0];
	let text = '';

	onMount(async () => {
		setGlobalApiBase(PUBLIC_API_BASE || 'http://0.0.0.0:3000/api');

		const res = await api.generate({ analysis });
		console.log(res);
		text = res.json.commands;
	});
</script>

<h1 class="text-3xl">INSPECTIFY!!</h1>

<div>
	<button
		on:click={async () => {
			const res = await api.generate({ analysis });
			console.log(res);
			text = res.json.commands;
		}}>Generate</button
	>
</div>
<Editor value={text} />
