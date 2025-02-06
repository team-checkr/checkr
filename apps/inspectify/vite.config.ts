import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import Icons from 'unplugin-icons/vite';
import path from 'path';
import { execSync } from 'child_process';

const metadata = JSON.parse(execSync('cargo metadata --format-version 1 --no-deps').toString()) as {
  packages: { name: string; version: string }[];
};

const version = metadata.packages.find((x) => x.name == 'inspectify')?.version;

process.env.PUBLIC_INSPECTIFY_VERSION = version;

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
