<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io';

  const io = useIo('Graph', {
    commands: 'skip',
    determinism: 'Deterministic',
  });
</script>

<Env {io}>
  <svelte:fragment slot="input">
    <StandardInput analysis="Graph" code="commands" {io} />
  </svelte:fragment>
  <svelte:fragment slot="output" let:output>
    <div class="relative">
      <div class="absolute inset-0 grid overflow-auto">
        <Network dot={output.dot || ''} />
      </div>
    </div>
  </svelte:fragment>
</Env>
