<script lang="ts">
  import { writable } from 'svelte/store';
  import { browser } from '$app/environment';
  import Editor from '$lib/components/Editor.svelte';
  import type { LtLResult, MarkerData } from 'chip-wasm';
  import Nav from '$lib/components/Nav.svelte';
  import Network from '$lib/components/Network.svelte';

  let program = `{ x := 2 }
x := x + 1
{ x = 2 }`;

  let result = writable<LtLResult>({
    parse_error: false,
    markers: [],
    buchi_dot: '',
    buchi_property_dot: '',
    gbuchi_property_dot: '',
    kripke_str: '',
    product_ba_dot: '',
  });
  let verifications = writable<MarkerData[]>([]);

  let parseError = writable(false);

  const STATES = ['idle', 'verifying', 'verified', 'error'];
  type State = (typeof STATES)[number];
  let state = writable<State>('idle');

  $: graphs = [
    { title: 'Kripke structure', dot: $result.kripke_str },
    { title: 'Buchi automaton', dot: $result.buchi_dot },
    { title: 'Generalized Buchi property', dot: $result.gbuchi_property_dot },
    { title: 'Buchi property', dot: $result.buchi_property_dot },
    { title: 'Product automaton', dot: $result.product_ba_dot },
  ];

  $: if (browser) {
    const run = async () => {
      parseError.set(false);
      const { default: init, parse_ltl } = await import('chip-wasm');
      await init();
      const res = parse_ltl(program);
      if (res.parse_error) parseError.set(true);
      result.set(res);
    };
    run().catch(console.error);
  }

  const prepareDot = (dot: string) =>
    dot
      .trim()
      .replaceAll('\n\n\n', '\n\n')
      .replaceAll(/"\]\[shape="doublecircle"/g, ' ⭐"')
      .replaceAll(/\[label=[^\]]+\]\[shape="point"]/g, '[label="."]')
      .replaceAll(/\]\[shape/g, ',shape');
</script>

<svelte:head>
  <title>Moka</title>
  <meta name="description" content="Moka" />
</svelte:head>

<Nav title="Moka" />

<div class="relative grid grid-cols-2 grid-rows-[2fr_auto] bg-slate-800">
  <Editor bind:value={program} markers={[...$result.markers, ...$verifications]} />
  <div class="flex flex-col text-white">
    {#each graphs as { title, dot }}
      {#if dot}
        <div class="flex flex-1 flex-col p-2">
          <h2 class="text-xl font-bold">{title}</h2>
          <div class="grid flex-1 grid-cols-2 overflow-auto rounded bg-slate-700">
            <pre class="p-4">{prepareDot(dot)}</pre>
            <div class="m-1 rounded bg-slate-900">
              <Network dot={prepareDot(dot)} />
            </div>
          </div>
        </div>
      {/if}
    {/each}
  </div>
  <div
    class="flex items-center p-2 text-2xl text-white transition duration-500 {$parseError
      ? 'bg-purple-600'
      : {
          idle: 'bg-gray-500',
          verifying: 'bg-yellow-500',
          verified: 'bg-green-500',
          error: 'bg-red-500',
        }[$state]}"
  >
    <span class="font-bold">
      {$parseError
        ? 'Parse error'
        : {
            idle: 'Idle',
            verifying: 'Verifying...',
            verified: 'Verified',
            error: 'Verification error',
          }[$state]}
    </span>
    <div class="flex-1" />
    <span class="text-xl">
      <!-- {#if !$parseError && $state == 'verified'}
          {#if $result.is_fully_annotated}
            The program is <b>fully annotated</b>
          {:else}
            The program is <b><i>not</i> fully annotated</b>
          {/if}
        {/if} -->
    </span>
  </div>
  <!-- <div>
          {#each result.assertions as triple}
              <pre class="p-4">{triple.smt}</pre>
          {/each}
      </div> -->
</div>
