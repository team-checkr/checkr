<script lang="ts">
	import { ce_shell } from '$lib/api';
	import { type Io } from '$lib/io';

	export let io: Io<ce_shell.Analysis>;

	const { outputState, validation } = io;
</script>

<div
	class="col-span-full col-start-2 row-start-2 h-4 text-sm transition {$outputState == 'Current'
		? $validation?.type == 'CorrectTerminated' || $validation?.type == 'CorrectNonTerminated'
			? 'bg-green-500'
			: 'bg-red-500'
		: 'bg-gray-500'}"
></div>
