/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

/**
 * vue-virtual-scroller 2.0.0-beta.8 TS deklarasyonu publish etmiyor; stub'lıyoruz.
 * RecycleScroller props'unu minimum geçirgenlikte tanımlıyoruz — strict mode'da
 * derlensin diye. Library API'si change ederse buraya dokunmamız gerekecek.
 */
declare module "vue-virtual-scroller" {
  import type { DefineComponent } from "vue";
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const RecycleScroller: DefineComponent<any, any, any>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const DynamicScroller: DefineComponent<any, any, any>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const DynamicScrollerItem: DefineComponent<any, any, any>;
}

declare module "vue-virtual-scroller/dist/vue-virtual-scroller.css";
