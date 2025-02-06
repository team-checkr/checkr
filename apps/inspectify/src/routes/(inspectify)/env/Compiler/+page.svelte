<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { Io } from '$lib/io.svelte';
  import InputOptions from '$lib/components/InputOptions.svelte';
  import DeterminismInput from '$lib/components/DeterminismInput.svelte';

  const io = new Io('Compiler', {
    commands: 'skip',
    determinism: 'Deterministic',
  });
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Compiler" code="commands" {io}>
      <InputOptions>
        <DeterminismInput input={io.input} />
      </InputOptions>
    </StandardInput>
  {/snippet}
  {#snippet outputView({ output })}
    <div class="relative">
      <div class="absolute inset-0 grid overflow-auto">
        <Network dot={output.dot || ''} />
      </div>
    </div>
  {/snippet}
</Env>
