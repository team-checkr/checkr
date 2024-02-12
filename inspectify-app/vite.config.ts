import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import Icons from 'unplugin-icons/vite';

// const commitHash =
//   process.env.GITHUB_REF_NAME ??
//   execSync("git describe --dirty").toString().trimEnd();
// process.env.INSPECTIFY_VERSION = commitHash;

export default defineConfig({
  plugins: [sveltekit(), Icons({ compiler: 'svelte' })],
});
