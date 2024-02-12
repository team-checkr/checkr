<script lang="ts">
  import { PUBLIC_API_BASE } from '$env/static/public';
  import { api, ce_shell, setGlobalApiBase, type ce_graph } from '$lib/api';
  import Editor from '$lib/components/Editor.svelte';
  import { onMount } from 'svelte';

  let analysis: ce_shell.Analysis = ce_shell.ANALYSIS[0];
  let text = '';

  const regenerate = async () => {
    const res = await api.generate({ analysis }).data;

    if (analysis == 'Graph') {
      const body = res.json as ce_graph.GraphInput;
      text = body.commands;
    }
  };

  onMount(async () => {
    setGlobalApiBase(PUBLIC_API_BASE || 'http://0.0.0.0:3000/api');

    await regenerate();
  });
</script>

<h1 class="text-3xl">INSPECTIFY!!</h1>

<div>
  <button on:click={regenerate}>Generate</button>
</div>
<Editor value={text} />
