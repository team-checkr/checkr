<script lang="ts">
  import Env from '$lib/components/Env.svelte';
  import StandardInput from '$lib/components/StandardInput.svelte';
  import { useIo } from '$lib/io';

  const io = useIo('Parse', { commands: 'skip' });
</script>

<Env {io}>
  <svelte:fragment slot="input">
    <StandardInput analysis="Parse" code="commands" {io} />
  </svelte:fragment>

  <svelte:fragment slot="output" let:output let:referenceOutput>
    <div class="relative">
      <div class="absolute inset-0 grid">
        <pre class="p-2"><code
            >{#if output}{output.pretty}{/if}</code
          ></pre>
      </div>
    </div>
  </svelte:fragment>
</Env>
