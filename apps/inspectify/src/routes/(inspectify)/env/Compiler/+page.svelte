<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { Io } from '$lib/io.svelte';
  import InputOptions from '$lib/components/InputOptions.svelte';
  import DeterminismInput from '$lib/components/DeterminismInput.svelte';
  import LevelInput from '$lib/components/LevelInput.svelte';

  const io = new Io('Compiler', {
    commands: 'skip',
    determinism: 'Deterministic',
    witness_mems: [],
    level: 7,
  });

  const LEVELS = [
    { n: 1, name: 'Assignment' },
    { n: 2, name: 'Sequencing' },
    { n: 3, name: 'Conditionals' },
    { n: 4, name: 'Loops' },
    { n: 5, name: 'Arrays' },
    { n: 6, name: 'Nondeterminism' },
    { n: 7, name: 'Composition' },
  ];
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Compiler" code="commands" {io}>
      <InputOptions>
        <LevelInput bind:level={io.level} {LEVELS} />
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
