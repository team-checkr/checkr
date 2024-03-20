import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import Icons from 'unplugin-icons/vite';
import path from 'path';

// const commitHash =
//   process.env.GITHUB_REF_NAME ??
//   execSync("git describe --dirty").toString().trimEnd();
// process.env.INSPECTIFY_VERSION = commitHash;

export default defineConfig({
  resolve: {
    alias: {
      'tailwind.config.ts': path.resolve(__dirname, 'tailwind.config.ts'),
    },
  },
  optimizeDeps: {
    include: [path.resolve(__dirname, 'tailwind.config.ts')],
  },
  server: {
    fs: {
      allow: ['./tailwind.config.ts'],
    },
  },
  plugins: [sveltekit(), Icons({ compiler: 'svelte' })],
});
