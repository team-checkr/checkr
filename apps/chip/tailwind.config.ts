import typography from '@tailwindcss/typography';
import forms from '@tailwindcss/forms';
import type { Config } from 'tailwindcss';
import resolveConfig from 'tailwindcss/resolveConfig';
export default resolveConfig({
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {},
  },
  plugins: [forms, typography],
} satisfies Config);
