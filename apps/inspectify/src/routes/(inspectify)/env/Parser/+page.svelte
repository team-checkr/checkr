<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io';

  const io = useIo('Parser', { commands: 'skip' });
</script>

<Env {io}>
  {#snippet inputView()}
    <StandardInput analysis="Parser" code="commands" {io} />
  {/snippet}

  {#snippet outputView({ output, referenceOutput })}
    <div class="relative">
      <div class="absolute inset-0 grid">
        <pre class="p-2"><code
            >{#if output}{output.pretty}{/if}</code
          ></pre>
      </div>
    </div>
  {/snippet}
</Env>
