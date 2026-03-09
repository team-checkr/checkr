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
  {#snippet outputView({ output, referenceOutput })}
    <div class="relative">
      <div class="absolute inset-0 grid grid-rows-2">
        <pre class="p-2 select-all"><code
            >{#if output}{output.assembly}{/if}</code
          ></pre>
      </div>
    </div>
  {/snippet}
</Env>
