<script lang="ts">
	import { createEventDispatcher, onDestroy, onMount } from 'svelte';
	import type * as Monaco from 'monaco-editor/esm/vs/editor/editor.api';

	export let value: string;

	let editor: Monaco.editor.IStandaloneCodeEditor;
	let monaco: typeof Monaco;
	let model: Monaco.editor.ITextModel;
	let editorContainer: HTMLElement;

	onMount(async () => {
		monaco = (await import('../monaco')).default;

		// const { PGCL_LANGUAGE_ID } = await import('./pgcl');
		// const { AYU_MIRAGE } = await import('../themes/ayu');

		editor = monaco.editor.create(editorContainer, {
			minimap: { enabled: false },
			lineNumbers: 'off',
			// theme: AYU_MIRAGE,
			scrollBeyondLastLine: false
		});
		model = monaco.editor.createModel(value, 'idk');
		editor.setModel(model);
		model.onDidChangeContent(() => {
			value = model.getValue();
		});
	});

	onDestroy(() => {
		monaco?.editor.getModels().forEach((model) => model.dispose());
		editor?.dispose();
	});

	$: if (model && typeof value == 'string' && model.getValue() != value) {
		model.setValue(value);
	}
</script>

<div class="h-96 w-full" bind:this={editorContainer} />

<style>
	.container {
		width: 100%;
		height: 600px;
	}
</style>
