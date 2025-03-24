<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type * as Monaco from 'monaco-editor/esm/vs/editor/editor.api';
  import type { MarkerData } from 'chip-wasm';
  import { theme } from '$lib/theme';
  import chipDark from '$lib/themes/dark.json';
  import chipLight from '$lib/themes/light.json';
  import type monacoT from '../monaco';

  export let value: string = '';
  export let markers: MarkerData[] = [];
  export let hoveredMarker: number | null = null;
  export let readOnly = false;

  let editor: Monaco.editor.IStandaloneCodeEditor;
  let monaco: typeof Monaco;
  let model: Monaco.editor.ITextModel;
  let editorContainer: HTMLElement;

  onMount(() => {
    let observer: ResizeObserver | void;
    const run = async () => {
      monaco = (await import('../monaco')).default;
      const { GCL_LANGUAGE_ID } = await import('../langs/gcl');

      monaco.editor.defineTheme('chip-dark', chipDark as any);
      monaco.editor.defineTheme('chip-light', chipLight as any);

      // const { AYU_MIRAGE } = await import('../themes/ayu');

      editor = monaco.editor.create(editorContainer, {
        fontSize: 24,
        minimap: { enabled: false },
        lineNumbers: 'on',
        // theme: AYU_MIRAGE,
        theme: 'chip-dark',
        scrollBeyondLastLine: false,
        language: GCL_LANGUAGE_ID,
        readOnly,
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
  let decorations: monacoT.editor.IEditorDecorationsCollection | null = null;
  $: if (model) {
    decorations = editor.createDecorationsCollection();
  }
  $: if (model && decorations) {
    const m2 = markers.map((m) => ({
      relatedInformation: m.relatedInformation?.map((r) => ({
        resource: model.uri,
        startLineNumber: r.span.startLineNumber,
        startColumn: r.span.startColumn,
        endLineNumber: r.span.endLineNumber,
        endColumn: r.span.endColumn,
        message: r.message,
      })),
      tags: (m.tags ?? []).map((t) => monaco.MarkerTag[t]),
      severity: monaco.MarkerSeverity[m.severity],
      message: m.message.split('\n')[0],
      startLineNumber: m.span.startLineNumber,
      startColumn: m.span.startColumn,
      endLineNumber: m.span.endLineNumber,
      endColumn: m.span.endColumn,
    }));
    monaco.editor.setModelMarkers(model, 'gcl', m2);
    decorations.set(
      markers.map((m) => ({
        options: {
          hoverMessage: {
            value: m.message.split('\n').slice(1).join('\n'),
            supportHtml: true,
            isTrusted: true,
          },
        },
        range: {
          startLineNumber: m.span.startLineNumber,
          startColumn: m.span.startColumn,
          endLineNumber: m.span.endLineNumber,
          endColumn: m.span.endColumn,
        },
      })),
    );
  }
  $: if (editor) {
    editor.updateOptions({ theme: $theme == 'dark' ? 'chip-dark' : 'chip-light' });
  }
  $: if (editor) {
    monaco.languages.registerHoverProvider('gcl', {
      provideHover(model, position, token) {
        const found = markers.findIndex((marker) => {
          const { column, lineNumber } = position;
          // check if positions is within the marker
          if (
            marker.span.startLineNumber <= lineNumber &&
            marker.span.endLineNumber >= lineNumber
          ) {
            if (marker.span.startColumn <= column && marker.span.endColumn >= column) {
              return true;
            }
          }
        });
        if (found >= 0) {
          hoveredMarker = found;
          const i = setInterval(() => {
            if (hoveredMarker != found) {
              clearInterval(i);
              return;
            }
            if (token.isCancellationRequested) {
              hoveredMarker = null;
              clearInterval(i);
            }
          }, 1000);
        } else {
          hoveredMarker = null;
        }
        token.onCancellationRequested(() => {
          console.log('cancel');
        });
        return null;
      },
    });
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
