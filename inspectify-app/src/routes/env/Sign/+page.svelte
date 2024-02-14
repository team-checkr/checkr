<script lang="ts">
  import { browser } from '$app/environment';
  import { api, ce_sign, type inspectify_api } from '$lib/api';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import ValidationIndicator from '$lib/components/ValidationIndicator.svelte';
  import { useIo } from '$lib/io';

  const io = useIo(
    'Sign',
    {
      commands: 'skip',
      assignment: { variables: {}, arrays: {} },
      determinism: { Case: 'Deterministic' },
    },
    { nodes: {}, initial_node: '', final_node: '' },
  );
  const { input, results } = io;
  $: output = $results.output;

  $: commands = $input.commands;
  $: determinism = $input.determinism.Case;
  $: dotPromise = api.gclDot({ determinism: { Case: determinism }, commands }).data;

  $: vars = [] as inspectify_api.endpoints.Target[];
  $: if (browser) {
    api.gclFreeVars(commands || 'skip').data.then((newVars) => {
      newVars.sort((a, b) => (a.name > b.name ? 1 : -1));
      vars = newVars;
    });
  }

  // TODO: update input signs when vars change

  const fmtSignOrSigns = (sign: ce_sign.semantics.Sign | ce_sign.semantics.Signs | void): string =>
    !sign
      ? '...'
      : Array.isArray(sign)
        ? sign.map(fmtSignOrSigns).join(' | ')
        : { Positive: '+', Zero: '0', Negative: '-' }[sign.Case];
</script>

<div class="grid grid-cols-[45ch_1fr_1fr] grid-rows-[1fr_auto]">
  <StandardInput analysis="Sign" code="commands" {io}>
    <h1 class="border-y p-2 pb-1 text-lg font-bold">Initial sign assignment</h1>
    <div class="grid grid-cols-[auto_repeat(3,1fr)] place-items-center">
      {#each vars.slice().sort((a, b) => (a.name > b.name ? 1 : -1)) as v}
        <div class="px-4 py-0.5 font-mono text-sm">
          {v.name}
        </div>
        {#each ce_sign.semantics.SIGN as sign}
          {#if v.kind == 'Variable'}
            <div>
              <label for="{v.name}-{sign.Case}">{fmtSignOrSigns(sign)}</label>
              <input
                type="radio"
                name={v.name}
                id="{v.name}-{sign.Case}"
                value={$input.assignment.variables[v.name].Case == sign.Case
                  ? $input.assignment.variables[v.name]
                  : sign}
                bind:group={$input.assignment.variables[v.name]}
              />
            </div>
          {:else if v.kind == 'Array'}
            <div>
              {fmtSignOrSigns(sign)}
            </div>
          {:else}
            <div>...</div>
          {/if}
        {/each}
      {/each}
    </div>
  </StandardInput>
  <div class="relative border-r">
    <div class="absolute inset-0 grid overflow-auto">
      {#if dotPromise}
        {#await dotPromise then { dot }}
          <Network {dot} />
        {/await}
      {/if}
    </div>
  </div>
  <div class="relative">
    <div class="absolute inset-0 overflow-auto">
      <div
        class="grid w-full grid-flow-dense [&_*]:border-t"
        style="grid-template-columns: repeat({vars.length + 1}, auto);"
      >
        <div class="border-none"></div>
        {#each vars as v}
          <div class="border-none text-center">{v.name}</div>
        {/each}
        {#each Object.entries(output.nodes) as [node, mems]}
          {#each mems as mem, idx}
            {#if idx == 0}
              <h2 class="px-2" style="grid-row: span {mems.length} / span {mems.length};">
                {node}
              </h2>
            {/if}
            {#each vars as v}
              <div class="px-2 py-0.5 font-mono text-sm">
                {v.name}: {v.kind == 'Array'
                  ? fmtSignOrSigns(mem.arrays[v.name])
                  : fmtSignOrSigns(mem.variables[v.name])}
              </div>
            {/each}
          {/each}
        {/each}
      </div>
    </div>
  </div>
  <ValidationIndicator {io} />
</div>
