<script lang="ts">
  import { browser } from '$app/environment';
  import { GCL } from '$lib/api';
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { Io } from '$lib/io.svelte';
  import { toSubscript } from '$lib/fmt';
  import ParsedInput from './ParsedInput.svelte';
  import InputOptions from '$lib/components/InputOptions.svelte';
  import InputOption from '$lib/components/InputOption.svelte';
  import DeterminismInput from '$lib/components/DeterminismInput.svelte';

  const io = new Io('Interpreter', {
    commands: 'skip',
    determinism: GCL.DETERMINISM[0],
    assignment: { variables: {}, arrays: {} },
    trace_length: 10,
    level: 7,
  });
  let vars = $derived(io.meta ?? []);

  $effect.pre(() => {
    if (browser) {
      for (const v of vars) {
        if (v.kind == 'Variable') {
          if (typeof io.input.assignment.variables[v.name] != 'number') {
            io.input.assignment.variables[v.name] = 0;
          }
        } else if (v.kind == 'Array') {
          if (!Array.isArray(io.input.assignment.arrays[v.name])) {
            io.input.assignment.arrays[v.name] = [0];
          }
        }
      }
    }
  });

  // TODO move

  let { level = $bindable() }: { level: number } = $props();

  const LEVELS = [
    { n: 1, name: 'Assignment' },
    { n: 2, name: 'Sequencing' },
    { n: 3, name: 'Conditionals' },
    { n: 4, name: 'Stuck' },
    { n: 5, name: 'Loops' },
    { n: 6, name: 'Nondeterminism' },
    { n: 7, name: 'Composition' },
  ];

  const currentName = $derived(LEVELS.find((l) => l.n === level)?.name ?? '');

</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Interpreter" code="commands" {io}>
      <InputOptions title="Initialization of variables and arrays">
        <div class="col-span-full grid grid-cols-[max-content_1fr] items-center gap-y-2 px-1 py-1">
          {#each vars.slice().sort((a, b) => (a.name > b.name ? 1 : -1)) as v}
            <div class="px-4 py-0.5 font-mono text-sm">
              {v.name}
            </div>
            <div class="w-full font-mono">
              {#if v.kind == 'Array'}
                <ParsedInput type="array" bind:value={io.input.assignment.arrays[v.name]} />
              {:else}
                <ParsedInput type="int" bind:value={io.input.assignment.variables[v.name]} />
              {/if}
            </div>
          {/each}
        </div>
      </InputOptions>    
      <InputOptions>
        <InputOption title="Number of steps">
          <div class="w-full font-mono">
            <ParsedInput type="int" bind:value={io.input.trace_length} />
          </div>
        </InputOption>
        <InputOption title="Level">
          <div class="flex flex-col gap-y-1">
            <div class="grid w-full grid-cols-7 gap-x-1 font-mono">
              {#each LEVELS as { n }}
                <button
                  onclick={() => (
                    io.input.level = n,
                    level = n
                  )}
                  class="rounded py-1 text-center text-xs transition {n <= level
                    ? 'bg-slate-500 text-white'
                    : 'bg-slate-800 text-slate-500'}"
                >
                  {n}
                </button>
              {/each}
            </div>
            <div class="text-xs text-slate-400">{currentName}</div>
          </div>
        </InputOption>
        <DeterminismInput input={io.input} />
      </InputOptions>
    </StandardInput>
  {/snippet}
  {#snippet outputView({ input: cachedInput, output, meta })}
    <div class="grid min-h-0 grid-cols-[auto_1fr]">
      <div class="flex min-h-0 flex-col border-t border-r bg-slate-900">
        <div class="flex items-center justify-between border-b border-slate-700 px-4 py-2">
          <div class="font-mono text-sm font-semibold tracking-wider text-slate-200 uppercase">
            Trace
          </div>
          {#if cachedInput.determinism == 'NonDeterministic'}
            <button
              type="button"
              class="rounded border border-slate-500 px-3 py-1 text-xs font-semibold tracking-wide text-slate-100 uppercase transition hover:bg-slate-800 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-slate-200"
              onclick={() => io.rerun()}
            >
              Change path
            </button>
          {/if}
        </div>
        <div class="overflow-auto">
          <div
            class="grid gap-x-4 px-4 py-2"
            style="grid-template-columns: max-content min-content repeat({Math.max(
              meta.length,
              1,
            )}, max-content);"
          >
            <div></div>
            <div></div>
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
              <div></div>
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
                <div></div>
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
                <div class="my-1 rounded-sm bg-blue-500 px-2 py-1 font-bold text-white">
                  Stopped after {output.trace.length} steps
                </div>
              {:else if output.termination == 'Terminated'}
                <div class="my-1 rounded-sm bg-green-500 px-2 py-1 font-bold text-white">
                  Terminated
                </div>
              {:else if output.termination == 'Stuck'}
                <div class="my-1 rounded-sm bg-red-500 px-2 py-1 font-bold text-white">Stuck</div>
              {/if}
            </div>
          </div>
        </div>
      </div>

      <div class="relative">
        <div class="absolute inset-0 grid overflow-auto">
          <Network dot={output.dot} />
        </div>
      </div>
    </div>
  {/snippet}
</Env>
