<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import type * as Monaco from 'monaco-editor/esm/vs/editor/editor.api';

  export let value: string = '';

  let editor: Monaco.editor.IStandaloneCodeEditor;
  let monaco: typeof Monaco;
  let model: Monaco.editor.ITextModel;
  let editorContainer: HTMLElement;

  onMount(() => {
    let observer: ResizeObserver | void;
    const run = async () => {
      monaco = (await import('../monaco')).default;
      const { GCL_LANGUAGE_ID } = await import('../langs/gcl');

      // const { AYU_MIRAGE } = await import('../themes/ayu');

      editor = monaco.editor.create(editorContainer, {
        minimap: { enabled: false },
        lineNumbers: 'off',
        // theme: AYU_MIRAGE,
        theme: 'vs-dark',
        scrollBeyondLastLine: false,
        language: GCL_LANGUAGE_ID,
      });
      model = monaco.editor.createModel(value, GCL_LANGUAGE_ID);
      editor.setModel(model);
      model.onDidChangeContent(() => {
        value = model.getValue();
      });

      observer = new ResizeObserver(() => editor.layout());
      observer.observe(editorContainer);
    };
    run();
    return () => observer?.disconnect();
  });

  onDestroy(() => {
    monaco?.editor.getModels().forEach((model) => model.dispose());
    editor?.dispose();
  });

  $: if (model && typeof value == 'string' && model.getValue() != value) {
    model.setValue(value);
  }
</script>

<div class="relative h-full w-full">
  <div class="absolute inset-0 overflow-hidden" bind:this={editorContainer} />
</div>

<style>
  .container {
    width: 100%;
    height: 600px;
  }
</style>
