/// <reference types="astro/client" />

interface ImportMetaEnv {
  readonly VERSION?: INSPECTIFY_VERSION;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
