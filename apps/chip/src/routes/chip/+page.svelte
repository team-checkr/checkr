<script lang="ts">
  import Editor from '$lib/components/Editor.svelte';
  import type { MarkerData, MarkerSeverity, ParseResult } from 'chip-wasm';
  import Nav from '$lib/components/Nav.svelte';
  import { untrack } from 'svelte';
  import Icon from '~icons/heroicons/check-badge';

  let program = $state(`{ true }
if
  false -> skip
fi
{ true }`);

  let result: ParseResult = $state({
    parse_error: false,
    prelude: '',
    assertions: [],
    markers: [],
    is_fully_annotated: false,
  });
  let verifications: MarkerData[] = $state([]);

  let parseError = $state(false);

  const Status = ['idle', 'verifying', 'verified', 'error'] as const;
  type Status = (typeof Status)[number];
  let status: Status = $state('idle');

  let parse: ((src: string) => ParseResult) | null = $state(null);

  $effect.pre(() => {
    const run = async () => {
      const { default: init, parse: parseFn } = await import('chip-wasm');
      await init();
      parse = parseFn;
    };
    run().catch(console.error);
  });

  $effect(() => {
    if (!parse) return;
    parseError = false;
    const res = parse(program);
    if (res.parse_error) parseError = true;
    result = res;
  });
  let runId = 0;
  $effect(() => {
    const thisResult: ParseResult = $state.snapshot(result) as ParseResult;
    let cancel = () => {};

    const run = async () => {
      const thisRun = ++runId;
      const z3 = await import('$lib/z3');
      verifications = [];
      status = 'verifying';
      let errors = false;
      for (const t of thisResult.assertions) {
        const { cancel: cancelZ3, result: resPromise } = z3.run(t.smt, {
          prelude: thisResult.prelude,
        });
        cancel = cancelZ3;
        const res = await resPromise;
        if (res == 'cancelled') return;

        const valid = res[res.length - 1].trim() === 'unsat';

        if (thisRun !== runId) {
          console.info('aborted', thisRun, runId, thisResult, res);
          return;
        }

        if (!valid) {
          errors = true;
          verifications = [
            ...untrack(() => verifications),
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
          ];
        }
      }
      if (errors) {
        status = 'error';
      } else {
        status = 'verified';
      }
    };
    run().catch(console.error);

    return () => {
      cancel();
    };
  });
</script>

<svelte:head>
  <title>Chip</title>
  <meta name="description" content="Chip" />
</svelte:head>

<Nav title="Chip" {Icon} />

<div class="relative grid grid-rows-[2fr_auto_auto] overflow-hidden bg-slate-800">
  <Editor bind:value={program} markers={[...result.markers, ...verifications]} />
  <div
    class="flex items-center p-2 text-2xl text-white transition duration-500 {parseError
      ? 'bg-purple-600'
      : {
          idle: 'bg-gray-500',
          verifying: 'bg-yellow-500',
          verified: 'bg-green-500',
          error: 'bg-red-500',
        }[status]}"
  >
    <span class="font-bold">
      {parseError
        ? 'Parse error'
        : {
            idle: 'Idle',
            verifying: 'Verifying...',
            verified: 'Verified',
            error: 'Verification error',
          }[status]}
    </span>
    <div class="flex-1"></div>
    <span class="text-xl">
      {#if !parseError && status == 'verified'}
        {#if result.is_fully_annotated}
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
