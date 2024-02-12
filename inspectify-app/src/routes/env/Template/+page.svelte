<script lang="ts">
  import StandardInput from '$lib/components/StandardInput.svelte';
  import ValidationIndicator from '$lib/components/ValidationIndicator.svelte';
  import { useIo } from '$lib/io';

  const io = useIo('Parse', { commands: 'skip' }, { pretty: 'skip' });
  const { input, results } = io;
  $: output = $results.output;
</script>

<div class="grid grid-cols-[45ch_1fr_1fr] grid-rows-[1fr_auto]">
  <StandardInput analysis="Parse" {io} />
  <ValidationIndicator {io} />
</div>
