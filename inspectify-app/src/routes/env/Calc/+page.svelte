<script lang="ts">
	import StandardInput from '$lib/components/StandardInput.svelte';
	import ValidationIndicator from '$lib/components/ValidationIndicator.svelte';
	import { useIo } from '$lib/io';

	const io = useIo('Calc', { commands: 'skip' }, { pretty: 'skip' });
	const { input, output } = io;
</script>

<div class="grid grid-cols-[45ch_1fr_1fr] grid-rows-[1fr_auto]">
	<StandardInput analysis="Calc" {io} />
	<ValidationIndicator {io} />
</div>
