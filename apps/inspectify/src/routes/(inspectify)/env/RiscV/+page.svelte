<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { Io } from '$lib/io.svelte';

  const io = new Io('RiscV', { commands: 'skip' });
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="RiscV" code="commands" {io} />
  {/snippet}
  {#snippet outputView({ output, referenceOutput, annotation })}
    <div class="grid grid-rows-[1fr_auto_1fr]">
      <div class="relative overflow-scroll flex-1">
        <div class="absolute inset-0">
          <pre class="p-2 select-all"><code
              >{#if output}{output.assembly}{/if}</code
            ></pre>
        </div>
      </div>
      <hr>
      {#if annotation}
      <div class="flex flex-col items-start mx-3">
        <h2 class="text-xl">Control</h2>
        <div class="grid gap-2 font-mono text-right pb-4" style="grid-template-columns: repeat({1 + Object.keys(annotation.regs).length}, auto);">
          <div class="font-bold">pc</div>
          {#each Object.keys(annotation.regs) as reg}
            <div class="font-bold">{reg}</div>
          {/each}
          <div>{annotation.pc}</div>
          {#each Object.values(annotation.regs) as reg}
            <div>{reg}</div>
          {/each}
        </div>
        <h2 class="text-xl">Variables</h2>
        <div class="grid gap-2 font-mono text-right pb-4" style="grid-template-columns: repeat(2, auto);">
          {#each Object.entries(annotation.variables) as reg}
            <div class="font-bold">{reg[0]}@{reg[1][0]}</div>
            <div>{reg[1][1]}</div>
          {/each}
        </div>
        <h2 class="text-xl">Memory</h2>
        <div class="grid gap-2 font-mono text-right pb-4" style="grid-template-columns: repeat(2, auto);">
          {#each Object.entries(annotation.memory) as reg}
            <div class="font-bold">{reg[0]}</div>
            <div>{reg[1]}</div>
          {/each}
        </div>
      </div>
      {/if}
    </div>
  {/snippet}
</Env>
