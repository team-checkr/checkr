<script lang="ts">
  import { browser } from '$app/environment';
  import { gcl } from '$lib/api';
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io';
  import { toSubscript } from '$lib/fmt';
  import ParsedInput from './ParsedInput.svelte';

  const io = useIo('Interpreter', {
    commands: 'skip',
    determinism: gcl.pg.DETERMINISM[0],
    assignment: { variables: {}, arrays: {} },
    trace_length: 10,
  });
  const { input, meta } = io;
  $: vars = $meta ?? [];

  $: if (browser) {
    for (const v of vars) {
      if (v.kind == 'Variable') {
        if (typeof $input.assignment.variables[v.name] != 'number') {
          $input.assignment.variables[v.name] = 0;
        }
      } else if (v.kind == 'Array') {
        if (!Array.isArray($input.assignment.arrays[v.name])) {
          $input.assignment.arrays[v.name] = [0];
        }
      }
    }
  }
</script>

<Env {io}>
  <svelte:fragment slot="input">
    <StandardInput analysis="Interpreter" code="commands" {io}>
      <h1 class="border-y p-2 pb-1 text-lg font-bold">Initial sign assignment</h1>
      <div class="grid grid-cols-[auto_1fr] place-items-center">
        {#each vars.slice().sort((a, b) => (a.name > b.name ? 1 : -1)) as v}
          <div class="px-4 py-0.5 font-mono text-sm">
            {v.name}
          </div>
          <div>
            {#if v.kind == 'Array'}
              <ParsedInput bind:value={$input.assignment.arrays[v.name]} />
            {:else}
              <ParsedInput bind:value={$input.assignment.variables[v.name]} />
            {/if}
          </div>
        {/each}
      </div></StandardInput
    >
  </svelte:fragment>
  <svelte:fragment slot="output" let:input={cachedInput} let:output let:meta>
    <div class="grid grid-rows-[1fr_minmax(auto,35vh)]">
      <div class="relative">
        <div class="absolute inset-0 grid overflow-auto">
          <Network dot={output.dot} />
        </div>
      </div>

      <div class="border-r border-t bg-slate-900">
        <div
          class="grid w-full gap-x-4 gap-y-0.5 px-4 py-2"
          style="grid-template-columns: max-content min-content repeat({meta.length}, max-content);"
        >
          <div />
          <div />
          <div
            class="border-b text-left font-mono font-bold"
            style="grid-column: span {meta.length}"
          >
            Memory
          </div>
          {#each ['Action', 'Node'] as name}
            <div class="text-left font-mono font-bold">
              {name}
            </div>
          {/each}

          {#each meta as v}
            <div class="text-center font-mono font-bold">
              {v.name}
            </div>
          {/each}

          {#each [{ action: '', node: output.initial_node, memory: cachedInput.assignment }, ...output.trace] as step}
            <div class="text-xs"><code>{step.action}</code></div>
            <div class="text-center">{toSubscript(step.node)}</div>
            {#each meta as v}
              <div class="px-1 text-right font-mono">
                {v.kind == 'Array'
                  ? JSON.stringify(step.memory.arrays[v.name])
                  : step.memory.variables[v.name]}
              </div>
            {/each}
          {/each}
        </div>
      </div>
    </div>
  </svelte:fragment>
</Env>
