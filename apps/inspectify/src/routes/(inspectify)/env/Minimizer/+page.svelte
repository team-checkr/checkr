<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import Network from '$lib/components/Network.svelte';
  import { Io } from '$lib/io.svelte';

  const io = new Io('Minimizer', { dfa: 'skip' });
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Minimizer" code="dfa" {io} />
  {/snippet}
  {#snippet outputView({ output, referenceOutput })}
    <div class="relative">
      <div class="absolute inset-0 grid overflow-auto">
        <Network dot={output.dfa || ''} />
      </div>
    </div>
  {/snippet}
</Env>
