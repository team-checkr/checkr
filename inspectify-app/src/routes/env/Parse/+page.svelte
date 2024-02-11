<script lang="ts">
	import StandardInput from '$lib/components/StandardInput.svelte';
	import ValidationIndicator from '$lib/components/ValidationIndicator.svelte';
	import { useIo } from '$lib/io';

	const io = useIo('Parse', { commands: 'skip' }, { pretty: '' });
	const output = io.output;
</script>

<div class="grid grid-cols-[45ch_1fr] grid-rows-[1fr_auto]">
	<StandardInput analysis="Parse" code="commands" {io} />
	<div class="relative">
		<div class="absolute inset-0 grid overflow-auto">
			<pre class="p-2"><code
					>{#if $output}{$output.pretty}{/if}</code
				></pre>
		</div>
	</div>
	<ValidationIndicator {io} />
</div>
