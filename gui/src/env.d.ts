/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue';
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>;
  export default component;
}

// Vue auto-import globals (used in <script setup> without explicit imports)
declare global {
  const ref: typeof import('vue')['ref'];
  const reactive: typeof import('vue')['reactive'];
  const computed: typeof import('vue')['computed'];
  const watch: typeof import('vue')['watch'];
  const readonly: typeof import('vue')['readonly'];
  const onMounted: typeof import('vue')['onMounted'];
  const onUnmounted: typeof import('vue')['onUnmounted'];
  const nextTick: typeof import('vue')['nextTick'];
  const useRoute: typeof import('vue-router')['useRoute'];
  const useRouter: typeof import('vue-router')['useRouter'];
}

export {};
