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
  const useVignetteList: typeof import('~/composables/useVignetteList')['useVignetteList'];
  const useProfiles: typeof import('~/composables/useProfiles')['useProfiles'];
  const useShortcutsDialog: typeof import('~/composables/useShortcutsDialog')['useShortcutsDialog'];
  const useToast: typeof import('~/composables/useToast')['useToast'];
  const useStoryBuilder: typeof import('~/composables/useStoryBuilder')['useStoryBuilder'];
  const select: typeof import('~/composables/useLocalDb')['select'];
  const execute: typeof import('~/composables/useLocalDb')['execute'];
}

export {};
