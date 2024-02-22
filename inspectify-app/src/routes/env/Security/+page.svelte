<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io';

  import ShieldExclamation from '~icons/heroicons/shield-exclamation';
  import LockClosed from '~icons/heroicons/lock-closed';

  const io = useIo('Security', { commands: 'skip', classification: {}, lattice: { rules: [] } });
</script>

<Env {io}>
  <svelte:fragment slot="input">
    <StandardInput analysis="Security" code="commands" {io} />
  </svelte:fragment>
  <svelte:fragment slot="output" let:output>
    <div>
      <h1 class="border-t bg-slate-900 p-2 text-2xl font-light italic">Computed flows</h1>
      <div class="grid min-h-0 grid-cols-[auto_1fr] p-2 gap-y-5">
        {#each [{ name: 'Allowed', rules: output.allowed }, { name: 'Actual', rules: output.actual }, { name: 'Violations', rules: output.violations }] as { name, rules }}
          <h2 class="mr-2 text-left font-bold">{name}:</h2>
          <div class="flex flex-wrap gap-1 font-mono items-center leading-tight">
            {#if rules.length == 0}
              <span class="shrink-0 italic text-sm opacity-75">None</span>
            {/if}
            {#each rules as rule (rule)}
              <span class="shrink-0 bg-white/5 py-0.5 px-1.5 rounded">{rule.from} → {rule.into}</span>
            {/each}
          </div>
        {/each}
        <div />
        <div class="flex">
          <div
            class="flex items-center space-x-1 rounded px-2 py-1 transition text-white {output.is_secure
              ? 'bg-green-500'
              : 'bg-red-500'}"
          >
            {#if output.is_secure}
              <LockClosed class="text-lg aspect-square" />
              <span>Secure</span>
            {:else}
              <ShieldExclamation class="text-lg aspect-square" />
              <span>Not Secure</span>
            {/if}
          </div>
        </div>
      </div>
    </div>
  </svelte:fragment>
</Env>
