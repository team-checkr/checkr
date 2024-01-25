<script lang="ts">
	import '../app.pcss';
	import { type Analysis, ANALYSIS } from '$lib/api';

	import CommandLineIcon from '~icons/heroicons/command-line';
	import PlayCircleIcon from '~icons/heroicons/play-circle';
	import QuestionMarkCircleIcon from '~icons/heroicons/question-mark-circle';

	let analysis: Analysis = ANALYSIS[0];
</script>

<div class="grid h-screen grid-rows-[auto_1fr]">
	<nav class="flex items-center bg-slate-900 px-2 text-slate-200">
		<a href="/" class="flex items-center space-x-2 p-2 pr-0 text-2xl font-thin italic">
			<div class="relative">
				<CommandLineIcon
					class="absolute inset-0 left-0.5 top-0.5 w-6 animate-pulse text-teal-500/50"
				/>
				<CommandLineIcon class="relative w-6" />
			</div>
			<span>Inspectify</span>
		</a>
		<!-- {
	  process.env.INSPECTIFY_VERSION && (
		<span class="text-xs place-self-end mb-2 text-slate-400 italic font-light">
		  {import.meta.env.INSPECTIFY_VERSION}
		</span>
	  )
	} -->

		<select class="ml-4 border-none bg-transparent py-0 text-right" bind:value={analysis}>
			<!-- on:input={(evt) => (analysis = Analysis.from_str(evt.target.value).unwrap())} -->
			{#each ANALYSIS as o}
				<option>{o}</option>
			{/each}
		</select>

		<div class="flex-1"></div>
		<a
			href="/"
			class="flex items-center space-x-1 p-2 text-sm font-semibold text-slate-300 transition hover:text-white"
		>
			<span>Analysis</span>
			<PlayCircleIcon class="w-4" />
		</a>

		<a
			href="/guide"
			class="flex items-center space-x-1 p-2 text-sm font-semibold text-slate-300 transition hover:text-white"
		>
			<span>Guide</span>
			<QuestionMarkCircleIcon class="w-4" />
		</a>
	</nav>

	<main>
		<slot />
	</main>
</div>
