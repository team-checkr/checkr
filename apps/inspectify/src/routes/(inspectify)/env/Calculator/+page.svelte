<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io.svelte';

  const io = useIo('Calculator', { expression: '1 + 2' });
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Calculator" code="expression" {io} />
  {/snippet}
  {#snippet outputView({ output, referenceOutput })}
    <div class="grid grid-cols-1">
      <div class="relative">
        <div class="absolute inset-0 flex flex-col border-r">
          <h1 class="border-t bg-slate-900 p-2 text-2xl font-light italic">Output</h1>
          {#if output.result}
            <h2 class="p-2 text-lg font-bold italic text-green-400">Result</h2>
            <pre class="overflow-auto rounded-md px-2 text-base">{output.result}</pre>
          {:else if output.error}
            <h2 class="p-2 text-lg font-bold italic text-orange-400">Evaluation error</h2>
            <pre class="overflow-auto rounded-md px-2 text-base">{output.error}</pre>
          {/if}
        </div>
      </div>
      <!-- <div class="relative">
          <div class="absolute inset-0 flex flex-col border-r">
            <h1 class="border-t bg-slate-900 p-2 text-2xl font-light italic">Reference</h1>
            {#if referenceOutput.result}
              <h2 class="p-2 text-lg font-bold italic text-green-400">Result</h2>
              <pre class="overflow-auto rounded-md px-2 text-base">{referenceOutput.result}</pre>
            {:else if referenceOutput.error}
              <h2 class="p-2 text-lg font-bold italic text-orange-400">Evaluation error</h2>
              <pre class="overflow-auto rounded-md px-2 text-base">{referenceOutput.error}</pre>
            {/if}
          </div>
        </div> -->
    </div>
  {/snippet}
</Env>
