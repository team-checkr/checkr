import { defineConfig } from "astro/config";
import wasm from "vite-plugin-wasm";

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
    server: {
      fs: {
        strict: false,
        // allow: ["../wasm/pkg"],
      },
    },
    build: {
      target: "esnext",
    },
    plugins: [wasm()],
  },
});
