<script lang="ts">
  import { browser } from '$app/environment';
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io.svelte';
  import type { SecurityAnalysis } from '$lib/api';

  import ShieldExclamation from '~icons/heroicons/shield-exclamation';
  import LockClosed from '~icons/heroicons/lock-closed';
  import InputOptions from '$lib/components/InputOptions.svelte';
  import InputOption from '$lib/components/InputOption.svelte';
  import ParsedInput from '../Interpreter/ParsedInput.svelte';

  const io = useIo('Security', { commands: 'skip', classification: {}, lattice: { rules: [] } });
  const { input, meta } = io;
  let targets = $derived($meta?.targets ?? []);
  let classes = $derived(
    $meta?.lattice.allowed
      .flatMap((a) => [a.from, a.into])
      .filter((v, i, a) => a.indexOf(v) === i) ?? [],
  );

  const stringify = (l: SecurityAnalysis.SecurityLatticeInput): string =>
    l.rules.map((a) => `${a.from} < ${a.into}`).join(', ');
  const parse = (s: string): SecurityAnalysis.SecurityLatticeInput | undefined => {
    const rules = s.split(',').map((r) => {
      const [from, into] = r.split(' < ');
      return { from: from?.trim(), into: into?.trim() };
    });
    if (rules.find((r) => !r.from || !r.into)) return void 0;
    return { rules };
  };

  $effect.pre(() => {
    if (browser && classes.length > 0) {
      for (const v of targets) {
        if (
          !(v.name in $input.classification) ||
          !classes.includes($input.classification[v.name])
        ) {
          $input.classification[v.name] = classes[Math.floor(Math.random() * classes.length)];
        }
      }
      const toDelete: string[] = [];
      for (const v of Object.keys($input.classification)) {
        if (!targets.find((t) => t.name === v)) {
          toDelete.push(v);
        }
      }
      for (const v of toDelete) {
        delete $input.classification[v];
      }
    }
  });
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Security" code="commands" {io}>
      <InputOptions title="Security Lattice">
        <InputOption title="Lattice">
          <div class="[&>input]:text-xs">
            <ParsedInput type="who knows" bind:value={$input.lattice} {stringify} {parse} />
          </div>
        </InputOption>
      </InputOptions>
      <InputOptions title="Classification for Variables and Arrays">
        <div class="col-span-full grid grid-cols-[max-content_1fr] items-center gap-y-2 px-1 py-1">
          {#each targets.slice().sort((a, b) => (a.name > b.name ? 1 : -1)) as v}
            <div class="px-4 py-0.5 font-mono text-sm">
              {v.name}
            </div>
            <div class="w-full font-mono">
              <select
                class="w-full rounded-sm border bg-transparent p-1"
                bind:value={$input.classification[v.name]}
              >
                {#each classes as c, index}
                  <option value={c} selected={index == 0} class="bg-slate-700">{c}</option>
                {/each}
              </select>
            </div>
          {/each}
        </div>
      </InputOptions>
    </StandardInput>
  {/snippet}
  {#snippet outputView({ output })}
    <div>
      <h1 class="border-t bg-slate-900 p-2 text-2xl font-light italic">Computed flows</h1>
      <div class="grid min-h-0 grid-cols-[auto_1fr] gap-y-5 p-2">
        {#each [{ name: 'Allowed', rules: output.allowed }, { name: 'Actual', rules: output.actual }, { name: 'Violations', rules: output.violations }] as { name, rules }}
          <h2 class="mr-2 text-left font-bold">{name}:</h2>
          <div class="flex flex-wrap items-center gap-1 font-mono leading-tight">
            {#if rules.length == 0}
              <span class="shrink-0 text-sm italic opacity-75">None</span>
            {/if}
            {#each rules as rule (rule)}
              <span class="shrink-0 rounded-sm bg-white/5 px-1.5 py-0.5"
                >{rule.from} → {rule.into}</span
              >
            {/each}
          </div>
        {/each}
        <div></div>
        <div class="flex">
          <div
            class="flex items-center space-x-1 rounded px-2 py-1 text-white transition {output.is_secure
              ? 'bg-green-500'
              : 'bg-red-500'}"
          >
            {#if output.is_secure}
              <LockClosed class="aspect-square text-lg" />
              <span>Secure</span>
            {:else}
              <ShieldExclamation class="aspect-square text-lg" />
              <span>Not Secure</span>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/snippet}
</Env>
