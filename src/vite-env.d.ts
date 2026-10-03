/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_COMPANY_ID?: string;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";

  const component: DefineComponent<
    Record<string, unknown>,
    Record<string, unknown>,
    unknown
  >;
  export default component;
}
