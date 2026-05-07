<template>
  <div class="app-shell">
    <MobileHeader @open="sidebarRef?.toggle()" />
    <div class="app-body">
      <AppSidebar ref="sidebarRef" />
      <main class="app-main">
        <RouterView />
      </main>
    </div>
    <ShortcutsDialog />
    <HostLivenessDialog />
  </div>
</template>

<script setup lang="ts">
import { RouterView } from 'vue-router';
import AppSidebar from '~/components/AppSidebar.vue';
import MobileHeader from '~/components/MobileHeader.vue';
import ShortcutsDialog from '~/components/ShortcutsDialog.vue';
import HostLivenessDialog from '~/components/HostLivenessDialog.vue';
import { useShortcutsDialog } from '~/composables/useShortcutsDialog';

const router = useRouter();

const sidebarRef = ref<{ toggle: () => void } | null>(null);

const { toggle: toggleShortcuts } = useShortcutsDialog();

const ALT_N_ROUTES: Record<string, string> = {
  h: '/',
  v: '/vignettes',
};

let awaitingAltN = false;

function navigateTo(route: string) {
  router.push(route);
}

function onGlobalKeydown(e: KeyboardEvent) {
  if (e.ctrlKey && e.shiftKey && (e.key === '/' || e.key === '?')) {
    e.preventDefault();
    toggleShortcuts();
    return;
  }

  if (e.altKey && e.key.toLowerCase() === 'n') {
    e.preventDefault();
    awaitingAltN = true;
    return;
  }

  if (awaitingAltN && !e.altKey) {
    awaitingAltN = false;
    const route = ALT_N_ROUTES[e.key.toLowerCase()];
    if (route) {
      e.preventDefault();
      navigateTo(route);
    }
  }
}

onMounted(() => document.addEventListener('keydown', onGlobalKeydown));
onUnmounted(() => document.removeEventListener('keydown', onGlobalKeydown));
</script>

<style scoped>
.app-shell {
  block-size: 100dvh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.app-body {
  display: flex;
  flex: 1;
  min-block-size: 0;
}

.app-main {
  flex: 1;
  min-block-size: 0;
  overflow-y: auto;
}
</style>
