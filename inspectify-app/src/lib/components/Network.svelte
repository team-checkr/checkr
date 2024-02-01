<script lang="ts">
	import { mirage } from 'ayu';

	export let dot: string;

	let container: HTMLDivElement | null = null;

	$: if (container) {
		const c = container;
		const run = async () => {
			const visPromise = import('vis-network/esnext');
			const vis = await visPromise;

			const data = vis.parseDOTNetwork(dot);

			new vis.Network(c, data, {
				interaction: { zoomView: false },
				nodes: {
					color: {
						background: mirage.ui.fg.hex(),
						border: mirage.ui.fg.hex(),
						highlight: mirage.ui.fg.brighten(1).hex()
						// background: '#666666',
						// border: '#8080a0',
						// highlight: '#80a0ff',
					},
					font: {
						color: 'white'
					},
					borderWidth: 1,
					shape: 'circle',
					size: 30
				},
				edges: {
					// color: '#D0D0FF',
					color: mirage.syntax.constant.hex(),
					font: {
						color: 'white',
						strokeColor: '#200020',
						face: 'Menlo, Monaco, "Courier New", monospace'
					}
				},
				autoResize: true
			});
		};

		requestAnimationFrame(() => run().catch(console.error));
	}
</script>

<div class="h-full w-full" bind:this={container}></div>
