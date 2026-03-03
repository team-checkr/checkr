<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { Io } from '$lib/io.svelte';

  const io = new Io('Minimizer', { dfa: 'skip' });
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Minimizer" code="dfa" {io} />
  {/snippet}
  {#snippet outputView({ output, referenceOutput })}
    <p>{output}</p>
  {/snippet}
</Env>
