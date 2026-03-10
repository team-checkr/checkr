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
    <div class="grid grid-rows-2">
      <div class="relative overflow-scroll">
        <div class="absolute inset-0">
          <pre class="p-2 select-all"><code
              >{#if output}{output.assembly}{/if}</code
            ></pre>
        </div>
      </div>
      <div>
        <hr>
        <pre><code>{annotation?.output}</code></pre>
      </div>
    </div>
  {/snippet}
</Env>
