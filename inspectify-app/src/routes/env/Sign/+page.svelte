<script lang="ts">
  import { derived } from 'svelte/store';
  import { browser } from '$app/environment';
  import { api, ce_sign, type inspectify_api } from '$lib/api';
  import Env from '$lib/components/Env.svelte';
  import Network from '$lib/components/Network.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io';

  const io = useIo('Sign', {
    commands: 'skip',
    assignment: { variables: {}, arrays: {} },
    determinism: { Case: 'Deterministic' },
  });
  const { input } = io;

  const commands = derived([input], ([input]) => input.commands);

  let vars: inspectify_api.endpoints.Target[] = [];
  $: if (browser) {
    api.gclFreeVars($commands || 'skip').data.then((newVars) => {
      newVars.sort((a, b) => (a.name > b.name ? 1 : -1));
      vars = newVars;
    });
  }

  const canonicalizeSign = (sign: ce_sign.semantics.Sign): ce_sign.semantics.Sign =>
    ce_sign.semantics.SIGN.find((s) => s.Case == sign.Case) || ce_sign.semantics.SIGN[0];

  // NOTE: we need to supply the initial signs to new variables, and we also
  // need to canonicalize the given signs, such that they will be the same in
  // the `bind:group`.
  $: if (browser) {
    for (const v of vars) {
      if (v.kind == 'Variable') {
        if (!$input.assignment.variables[v.name]) {
          $input.assignment.variables[v.name] = ce_sign.semantics.SIGN[0];
        } else if (
          $input.assignment.variables[v.name] !=
          canonicalizeSign($input.assignment.variables[v.name])
        ) {
          $input.assignment.variables[v.name] = canonicalizeSign(
            $input.assignment.variables[v.name],
          );
        }
      } else if (v.kind == 'Array') {
        if (!$input.assignment.arrays[v.name]) {
          $input.assignment.arrays[v.name] = [ce_sign.semantics.SIGN[0]];
        } else if (
          $input.assignment.arrays[v.name].some((sign) => sign != canonicalizeSign(sign))
        ) {
          $input.assignment.arrays[v.name] = $input.assignment.arrays[v.name].map(canonicalizeSign);
        }
      }
    }
  }

  const fmtSignOrSigns = (sign: ce_sign.semantics.Sign | ce_sign.semantics.Signs | void): string =>
    !sign
      ? '...'
      : Array.isArray(sign)
        ? sign.map(fmtSignOrSigns).join(' | ')
        : { Positive: '+', Zero: '0', Negative: '-' }[sign.Case];

  const subscriptMap: Record<string, string | void> = {
    '0': '₀',
    '1': '₁',
    '2': '₂',
    '3': '₃',
    '4': '₄',
    '5': '₅',
    '6': '₆',
    '7': '₇',
    '8': '₈',
    '9': '₉',
  };
  const toSubscript = (str: string) =>
    str
      .split('')
      .map((char) => subscriptMap[char] || char)
      .join('');

  type CharClass = 'Fst' | 'Alp' | 'Num' | 'Oth' | 'Lst';

  const classifyChar = (c: string): CharClass =>
    [
      /[a-zA-Z]/.test(c) && 'Alp',
      /\d/.test(c) && 'Num',
      c === '▷' && 'Fst',
      c === '◀' && 'Lst',
      'Oth',
    ].find(Boolean) as CharClass;

  const naturalSort = (a: string, b: string) => {
    const aC = Array.from(a).map(classifyChar);
    const bC = Array.from(b).map(classifyChar);

    for (let i = 0; i < Math.min(aC.length, bC.length); i++) {
      const [x, y] = [aC[i], bC[i]];
      if (x === y) continue;

      if (x === 'Fst') return -1;
      if (y === 'Fst') return 1;
      if (x === 'Lst') return 1;
      if (y === 'Lst') return -1;
      if (x === 'Alp' && y === 'Num') return -1;
      if (x === 'Num' && y === 'Alp') return 1;
      if (x === 'Alp' && y === 'Oth') return -1;
      if (x === 'Oth' && y === 'Alp') return 1;
      if (x === 'Num' && y === 'Oth') return -1;
      if (x === 'Oth' && y === 'Num') return 1;
    }

    return aC.length - bC.length;
  };

  const sortNodes = <T,>(nodes: [string, T][]): [string, T][] =>
    nodes.sort(([a], [b]) => naturalSort(a, b));
</script>

<Env {io}>
  <svelte:fragment slot="input">
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
                  value={sign}
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
  </svelte:fragment>

  <svelte:fragment slot="output" let:output>
    <div class="grid grid-cols-[auto_1fr]">
      <div class="border-r border-t bg-slate-900">
        <div
          class="grid w-full grid-flow-dense [&_*]:border-t"
          style="grid-template-columns: min-content repeat({vars.length}, max-content);"
        >
          <div class="border-none"></div>
          {#each vars as v}
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
              {#each vars as v}
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
  </svelte:fragment>
</Env>
