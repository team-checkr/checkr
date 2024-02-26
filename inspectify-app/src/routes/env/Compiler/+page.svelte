<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { GCL } from '$lib/api';
  import { useIo } from '$lib/io';
  import InputOptions from '$lib/components/InputOptions.svelte';
  import InputOption from '$lib/components/InputOption.svelte';

  const io = useIo('Compiler', {
    commands: 'skip',
    determinism: 'Deterministic',
  });
  const { input } = io;
</script>

<Env {io}>
  <svelte:fragment slot="input">
    <StandardInput analysis="Compiler" code="commands" {io}>
      <InputOptions>
        <InputOption title="Determinism">
          <div class="grid grid-cols-2 gap-x-2">
            {#each GCL.DETERMINISM as determinism}
              <div
                class="flex items-center justify-center rounded text-sm transition {$input.determinism ==
                determinism
                  ? 'bg-slate-500'
                  : 'bg-slate-800'}"
              >
                <label for="determinism-{determinism}" class="cursor-pointer px-2 py-1">
                  {determinism}
                </label>
                <input
                  class="hidden"
                  type="radio"
                  id="determinism-{determinism}"
                  name="determinism"
                  value={determinism}
                  bind:group={$input.determinism}
                />
              </div>
            {/each}
          </div>
        </InputOption>
      </InputOptions>
    </StandardInput>
  </svelte:fragment>
  <svelte:fragment slot="output" let:output>
    <div class="relative">
      <div class="absolute inset-0 grid overflow-auto">
        <Network dot={output.dot || ''} />
      </div>
    </div>
  </svelte:fragment>
</Env>
