<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io';

  const io = useIo('Parse', { commands: 'skip' });
</script>

<Env {io}>
  <svelte:fragment slot="input">
    <StandardInput analysis="Parse" {io} />
  </svelte:fragment>
  <svelte:fragment slot="output" let:output let:referenceOutput></svelte:fragment>
</Env>
