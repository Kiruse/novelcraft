<template>
  <div class="app-shell">
    <MobileHeader @open="sidebarRef?.toggle()" />
    <div class="app-body">
      <AppSidebar
        ref="sidebarRef"
        :user="currentUser"
        :sessions="sessions"
      />
      <main class="app-main">
        <NuxtPage />
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useCurrentUser } from '~/composables/useCurrentUser';

const { currentUser, sessions } = useCurrentUser();

const sidebarRef = ref<{ toggle: () => void } | null>(null);
</script>

<style scoped>
.app-shell {
  min-block-size: 100dvh;
  display: flex;
  flex-direction: column;
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
