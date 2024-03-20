<script lang="ts">
  import JsonViewInner from './JSONViewInner.svelte';

  export let json: any;

  let raw = false;
</script>

<div class="relative w-full p-1">
  <div class="absolute right-1 top-1">
    <label for="raw">Raw</label>
    <input type="checkbox" id="raw" bind:checked={raw} />
  </div>
  {#if raw}
    <pre class="font-mono text-slate-200">{JSON.stringify(json, null, 2)}</pre>
  {:else}
    <JsonViewInner {json} />
  {/if}
</div>
