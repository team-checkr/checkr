import { defineConfig } from "astro/config";

// https://astro.build/config
import tailwind from "@astrojs/tailwind";

// https://astro.build/config
import react from "@astrojs/react";

// https://astro.build/config
import mdx from "@astrojs/mdx";

// https://astro.build/config
import compress from "astro-compress";

// https://astro.build/config
export default defineConfig({
  server: {
    port: 3001,
  },
  integrations: [tailwind(), react(), mdx(), compress()],
  build: {
    format: "file",
  },
  vite: {
    build: {
      target: "esnext",
    },
  },
});
