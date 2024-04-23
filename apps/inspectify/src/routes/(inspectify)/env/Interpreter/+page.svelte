<script lang="ts">
  import { browser } from '$app/environment';
  import { GCL } from '$lib/api';
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io';
  import { toSubscript } from '$lib/fmt';
  import ParsedInput from './ParsedInput.svelte';
  import InputOptions from '$lib/components/InputOptions.svelte';
  import InputOption from '$lib/components/InputOption.svelte';
  import DeterminismInput from '$lib/components/DeterminismInput.svelte';

  const io = useIo('Interpreter', {
    commands: 'skip',
    determinism: GCL.DETERMINISM[0],
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
      <InputOptions title="Initialization of variables and arrays">
        <div class="col-span-full grid grid-cols-[max-content_1fr] items-center gap-y-2 px-1 py-1">
          {#each vars.slice().sort((a, b) => (a.name > b.name ? 1 : -1)) as v}
            <div class="px-4 py-0.5 font-mono text-sm">
              {v.name}
            </div>
            <div class="w-full font-mono">
              {#if v.kind == 'Array'}
                <ParsedInput type="array" bind:value={$input.assignment.arrays[v.name]} />
              {:else}
                <ParsedInput type="int" bind:value={$input.assignment.variables[v.name]} />
              {/if}
            </div>
          {/each}
        </div>
      </InputOptions>
      <InputOptions>
        <InputOption title="Number of steps">
          <div class="w-full font-mono">
            <ParsedInput type="int" bind:value={$input.trace_length} />
          </div>
        </InputOption>
        <DeterminismInput {input} />
      </InputOptions>
    </StandardInput>
  </svelte:fragment>
  <svelte:fragment slot="output" let:input={cachedInput} let:output let:meta>
    <div class="grid min-h-0 grid-cols-[auto_1fr]">
      <div class="overflow-auto border-r border-t bg-slate-900">
        <div
          class="grid gap-x-4 px-4 py-2"
          style="grid-template-columns: max-content min-content repeat({Math.max(
            meta.length,
            1,
          )}, max-content);"
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

          {#if meta.length == 0}
            <div />
          {/if}
          {#each meta as v}
            <div class="text-center font-mono font-bold">
              {v.name}
            </div>
          {/each}

          {#each [{ action: '', node: output.initial_node, memory: cachedInput.assignment }, ...output.trace] as step}
            <div class="line-clamp-1 max-w-[25ch] text-sm">
              <code>{step.action}</code>
            </div>
            <div class="text-center">{toSubscript(step.node)}</div>
            {#if meta.length == 0}
              <div />
            {/if}
            {#each meta as v}
              <div class="px-1 text-right font-mono text-slate-300">
                {v.kind == 'Array'
                  ? JSON.stringify(step.memory.arrays[v.name])
                  : step.memory.variables[v.name]}
              </div>
            {/each}
          {/each}
          <div class="flex">
            {#if output.termination == 'Running'}
              <div class="my-1 rounded bg-blue-500 px-2 py-1 font-bold text-white">
                Stopped after {output.trace.length} steps
              </div>
            {:else if output.termination == 'Terminated'}
              <div class="my-1 rounded bg-green-500 px-2 py-1 font-bold text-white">Terminated</div>
            {:else if output.termination == 'Stuck'}
              <div class="my-1 rounded bg-red-500 px-2 py-1 font-bold text-white">Stuck</div>
            {/if}
          </div>
        </div>
      </div>

      <div class="relative">
        <div class="absolute inset-0 grid overflow-auto">
          <Network dot={output.dot} />
        </div>
      </div>
    </div>
  </svelte:fragment>
</Env>
