<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { Io } from '$lib/io.svelte';
  import Graphviz from '$lib/components/Graphviz.svelte';

  const io = new Io('Minimizer', { dfa: 'skip' });
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Minimizer" code="dfa" {io} />
  {/snippet}
  {#snippet outputView({ output, referenceOutput })}
    <div class="flex flex-col h-full w-full">

      <div class="flex flex-col flex-1 min-h-0">
        <div class="border-t bg-slate-900 p-2 flex items-center gap-4">
          <h1 class="text-2xl font-light italic">Original</h1>
            {#if !output.deterministic}
              <span class="text-red-400 text-sm rounded-md font-bold italic">Nondeterministic</span>
            {/if}
        </div>
        <div class="relative flex-1">
          <div class="absolute inset-0">
            <Graphviz dot={output.dot || ''} />
          </div>
        </div>
      </div>

      {#if output.minimized_dot}
        <div class="flex flex-col flex-1 min-h-0">
          <h1 class="border-t bg-slate-900 p-2 text-2xl font-light italic">Minimized</h1>
          <div class="relative flex-1">
            <div class="absolute inset-0">
              <Graphviz dot={output.minimized_dot} />
            </div>
          </div>
        </div>
      {/if}

    </div>
  {/snippet}
</Env>
