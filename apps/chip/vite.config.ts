import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import Icons from 'unplugin-icons/vite';
import wasm from 'vite-plugin-wasm';
import wasmPack from './wasm-pack-plugin';

export default defineConfig({
	plugins: [
		sveltekit(),
		Icons({ compiler: 'svelte' }),
		wasm(),
		wasmPack({
			crates: ['../../crates/chip-wasm/']
		})
	],
	server: {
		fs: {
			allow: ['../../crates/chip-wasm/pkg/']
		}
	}
});
