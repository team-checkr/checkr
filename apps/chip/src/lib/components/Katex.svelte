<script lang="ts">
  import katex from 'katex';

  interface Props {
    math: string;
    displayMode?: boolean;
  }

  let { math, displayMode = false }: Props = $props();

  const options = {
    displayMode: displayMode,
    throwOnError: false,
  };

  let katexString = $derived(katex.renderToString(math, options));
</script>

<svelte:head>
  <link
    rel="stylesheet"
    href="https://cdn.jsdelivr.net/npm/katex@0.12.0/dist/katex.min.css"
    integrity="sha384-AfEj0r4/OFrOo5t7NnNe46zW/tFgW6x/bCJG8FqQCEo3+Aro6EYUG4+cU+KJWu/X"
    crossorigin="anonymous"
  />
</svelte:head>

{@html katexString}
