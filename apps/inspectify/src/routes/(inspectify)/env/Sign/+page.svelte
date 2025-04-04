<script lang="ts">
  import { browser } from '$app/environment';
  import { SignAnalysis } from '$lib/api';
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { Io } from '$lib/io.svelte';
  import { sortNodes, toSubscript } from '$lib/fmt';
  import InputOptions from '$lib/components/InputOptions.svelte';
  import DeterminismInput from '$lib/components/DeterminismInput.svelte';
  import InclusionCheckbox from '$lib/components/InclusionCheckbox.svelte';

  const io = new Io('Sign', {
    commands: 'skip',
    assignment: { variables: {}, arrays: {} },
    determinism: 'Deterministic',
  });

  let vars = $derived(io.meta ?? []);

  // NOTE: we need to supply the initial signs to new variables
  $effect.pre(() => {
    if (browser) {
      for (const v of vars) {
        if (v.kind == 'Variable') {
          if (!io.input.assignment.variables[v.name]) {
            io.input.assignment.variables[v.name] = SignAnalysis.SIGN[0];
          }
        } else if (v.kind == 'Array') {
          if (!io.input.assignment.arrays[v.name]) {
            io.input.assignment.arrays[v.name] = [SignAnalysis.SIGN[0]];
          }
        }
      }
    }
  });

  const fmtSignOrSigns = (sign: SignAnalysis.Sign | SignAnalysis.Sign[] | void): string =>
    !sign
      ? '...'
      : Array.isArray(sign)
        ? sign.map(fmtSignOrSigns).join(' | ')
        : { Positive: '+', Zero: '0', Negative: '-' }[sign];
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Sign" code="commands" {io}>
      <InputOptions title="Initial sign assignment">
        <div class="col-span-full grid w-full grid-cols-[auto_repeat(3,1fr)] place-items-center">
          {#each vars.slice().sort((a, b) => (a.name > b.name ? 1 : -1)) as v}
            <div class="px-4 py-0.5 font-mono text-sm">
              {v.name}
            </div>
            {#each SignAnalysis.SIGN as sign}
              {#if v.kind == 'Variable'}
                <div>
                  <label for="{v.name}-{sign}">{fmtSignOrSigns(sign)}</label>
                  <input
                    type="radio"
                    name={v.name}
                    id="{v.name}-{sign}"
                    value={sign}
                    bind:group={io.input.assignment.variables[v.name]}
                  />
                </div>
              {:else if v.kind == 'Array'}
                <div>
                  <label for="{v.name}-{sign}">{fmtSignOrSigns(sign)}</label>
                  <InclusionCheckbox
                    name={v.name}
                    id="{v.name}-{sign}"
                    value={sign}
                    bind:array={io.input.assignment.arrays[v.name]}
                  />
                </div>
              {:else}
                <div>...</div>
              {/if}
            {/each}
          {/each}
        </div>
      </InputOptions>
      <InputOptions>
        <DeterminismInput input={io.input} />
      </InputOptions>
    </StandardInput>
  {/snippet}

  {#snippet outputView({ output, meta })}
    <div class="grid grid-cols-[auto_1fr]">
      <div class="border-r border-t bg-slate-900">
        <div
          class="**:border-t grid w-full grid-flow-dense"
          style="grid-template-columns: min-content repeat({meta.length}, max-content);"
        >
          <div class="border-none"></div>
          {#each meta as v}
            <div class="border-none px-6 text-center font-mono font-bold">{v.name}</div>
          {/each}
          {#each sortNodes(Object.entries(output.nodes)) as [node, mems]}
            {#each mems as mem, idx}
              {#if idx == 0}
                <h2
                  class="px-3 text-left font-bold"
                  style="grid-row: span {mems.length} / span {mems.length};"
                >
                  {toSubscript(node)}
                </h2>
              {/if}
              {#each meta as v}
                <div class="px-2 py-0.5 text-center font-mono text-sm">
                  {v.kind == 'Array'
                    ? fmtSignOrSigns(mem.arrays[v.name])
                    : fmtSignOrSigns(mem.variables[v.name])}
                </div>
              {/each}
            {/each}
          {/each}
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
