<script lang="ts">
  import { writable } from 'svelte/store';
  import { browser } from '$app/environment';
  import Editor from '$lib/components/Editor.svelte';
  import type { MarkerData, MarkerSeverity, ParseResult } from 'chip-wasm';

  let program = `// {a=A}
// if a > 0 -> a := a + 1
// [] a = 0 -> a := 1
// [] a < 0 -> a := a {a>A}
// fi
// {a>A} ;

{n >= 0}
i := 0 ; sum := 0 ;
do[i <= n & sum = i * (i-1)/2]
  i < n ->
    sum := sum + i ;
    i := i + 1
od
{sum = n * (n-1)/2}`;

  let result = writable<ParseResult>({
    parse_error: false,
    prelude: '',
    assertions: [],
    markers: [],
    is_fully_annotated: false,
  });
  let verifications = writable<MarkerData[]>([]);

  let parseError = writable(false);

  const STATES = ['idle', 'verifying', 'verified', 'error'];
  type State = (typeof STATES)[number];
  let state = writable<State>('idle');

  $: if (browser) {
    const run = async () => {
      parseError.set(false);
      const { default: init, parse } = await import('chip-wasm');
      await init();
      const res = parse(program);
      if (res.parse_error) parseError.set(true);
      result.set(res);
    };
    run().catch(console.error);
  }
  let runId = 0;
  $: if (browser) {
    const run = async () => {
      const thisRun = ++runId;
      const z3 = await import('$lib/z3');
      verifications.set([]);
      state.set('verifying');
      let errors = false;
      for (const t of $result.assertions) {
        const res = await z3.run(t.smt, { prelude: $result.prelude });
        const valid = res[res.length - 1].trim() === 'unsat';

        if (thisRun !== runId) {
          console.log('aborted', thisRun, runId, result, res);
          return;
        }

        if (!valid) {
          errors = true;
          verifications.update((res) => [
            ...res,
            {
              severity: 'Error',
              tags: [],
              message: t.text ? t.text : 'Verification failed',
              span: t.span,
              relatedInformation: [],
            },
            ...(t.related
              ? [
                  {
                    severity: 'Info' as MarkerSeverity,
                    tags: [],
                    message: t.related[0],
                    span: t.related[1],
                    relatedInformation: [],
                  },
                ]
              : []),
          ]);
        }
      }
      if (errors) {
        state.set('error');
      } else {
        state.set('verified');
      }
    };
    run().catch(console.error);
  }
</script>

<svelte:head>
  <title>Chip</title>
  <meta name="description" content="Chip" />
</svelte:head>

<div class="relative grid grid-rows-[2fr_auto_auto] overflow-hidden bg-slate-800">
  <Editor bind:value={program} markers={[...$result.markers, ...$verifications]} />
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
      {#if !$parseError && $state == 'verified'}
        {#if $result.is_fully_annotated}
          The program is <b>fully annotated</b>
        {:else}
          The program is <b><i>not</i> fully annotated</b>
        {/if}
      {/if}
    </span>
  </div>
  <!-- <div>
		{#each result.assertions as triple}
			<pre class="p-4">{triple.smt}</pre>
		{/each}
	</div> -->
</div>
