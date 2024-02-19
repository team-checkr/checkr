<script lang="ts">
  import { gcl } from '$lib/api';
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io';
  import { toSubscript } from '$lib/fmt';

  const io = useIo('Interpreter', {
    commands: 'skip',
    determinism: gcl.pg.DETERMINISM[0],
    assignment: { variables: {}, arrays: {} },
    trace_length: 10,
  });
</script>

<Env {io}>
  <svelte:fragment slot="input">
    <StandardInput analysis="Interpreter" code="commands" {io} />
  </svelte:fragment>
  <svelte:fragment slot="output" let:input let:output let:referenceOutput>
    <div class="grid grid-rows-[1fr_minmax(auto,_50vh)]">
      <div class="relative">
        <div class="absolute inset-0 grid overflow-auto">
          <Network dot={output.dot} />
        </div>
      </div>

      <div class="border-r border-t bg-slate-900">
        <div
          class="grid w-full"
          style="grid-template-columns: min-content min-content repeat(1, max-content);"
        >
          {#each ['Action', 'Node', 'Memory'] as name}
            <div class="border-none px-4 text-center font-mono font-bold">
              {name}
            </div>
          {/each}

          {#each [{ action: '', node: '', memory: input.assignment }, ...output.trace] as step}
            <div class="text-xs"><code>{step.action}</code></div>
            <div class="text-center">{toSubscript(step.node)}</div>
            <div><code>{JSON.stringify(step.memory)}</code></div>
          {/each}
        </div>
      </div>
    </div>
  </svelte:fragment>
</Env>
