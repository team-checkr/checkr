<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { Io } from '$lib/io.svelte';

  const io = new Io('Parser', { commands: 'skip' });
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Parser" {io} />
  {/snippet}
  {#snippet outputView({ output, referenceOutput })}{/snippet}
</Env>
