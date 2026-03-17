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
  {#snippet outputView({ output, annotation })}
    <div class="grid grid-rows-[1fr_1fr]">
      <div class="flex flex-col">
        <h1 class="border-t bg-slate-900 p-2 text-2xl font-light italic">Assembly</h1>
        <div class="relative flex-1 overflow-auto">
          <div class="absolute inset-0">
            <pre class="p-2 select-all"><code
                >{#if output}{output.assembly}{/if}</code
              ></pre>
          </div>
        </div>
      </div>
      <div class="flex flex-col">
        <h1 class="border-t bg-slate-900 p-2 text-2xl font-light italic">Execution</h1>
        <div class="relative flex-1 overflow-auto">
          <div class="absolute inset-0 mx-3 flex flex-col items-start gap-4 py-4">
            {#if annotation}
              <div class="border">
                <h2 class="bg-slate-900 px-2 py-1 text-xl font-light">Control</h2>
                <div
                  class="grid text-right font-mono"
                  style="grid-template-columns: repeat({1 +
                    Object.keys(annotation.regs).length}, auto);"
                >
                  {#each ['pc', ...Object.keys(annotation.regs)] as reg}
                    <div class="bg-slate-700 px-2 text-left text-lg">{reg}</div>
                  {/each}
                  {#each [annotation.pc, ...Object.values(annotation.regs)] as reg}
                    <div class="px-2">{reg}</div>
                  {/each}
                </div>
              </div>
              <div class="grid grid-cols-2 gap-8">
                <div class="border">
                  <h2 class="bg-slate-900 px-2 py-1 text-xl font-light">Variables</h2>
                  <div class="grid grid-cols-3 text-right font-mono">
                    <div class="bg-slate-700 px-2 text-left font-sans text-lg">Label</div>
                    <div class="bg-slate-700 px-2 text-left font-sans text-lg">Location</div>
                    <div class="bg-slate-700 px-2 text-left font-sans text-lg">Value</div>
                    {#each Object.entries(annotation.variables) as reg}
                      <div class="px-2 font-bold">{reg[0]}</div>
                      <div class="px-2">{reg[1][0]}</div>
                      <div class="px-2">{reg[1][1]}</div>
                    {/each}
                  </div>
                </div>
                <div class="border">
                  <h2 class="bg-slate-900 px-2 py-1 text-xl font-light">Memory</h2>
                  <div class="grid grid-cols-2 text-right font-mono">
                    <div class="bg-slate-700 px-2 text-left font-sans text-lg">Location</div>
                    <div class="bg-slate-700 px-2 text-left font-sans text-lg">Value</div>
                    {#each Object.entries(annotation.memory) as reg}
                      <div class="px-2 font-bold">{reg[0]}</div>
                      <div class="px-2">{reg[1]}</div>
                    {/each}
                  </div>
                </div>
              </div>
            {:else}
              <i>Program couldn't execute</i>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/snippet}
</Env>
