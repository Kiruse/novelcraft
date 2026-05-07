<template>
  <Teleport to="body">
    <div v-if="visible" class="dialog-overlay" @click.self="dismiss">
      <div class="dialog-panel">
        <div class="dialog-header">
          <h2 class="dialog-title">Model hosts unreachable</h2>
          <button class="dialog-close" @click="dismiss">&times;</button>
        </div>

        <div class="dialog-body">
          <p class="dialog-description">
            The following model host{{ unreachableHosts.length > 1 ? 's are' : ' is' }} not responding:
          </p>

          <ul class="host-list">
            <li v-for="host in unreachableHosts" :key="host.url" class="host-item">
              <span class="host-url">{{ host.url }}</span>
              <span class="host-error">{{ host.error }}</span>
            </li>
          </ul>

          <p class="dialog-hint">
            Check that your LLM server is running and the URL is correct.
          </p>
        </div>

        <div class="dialog-footer">
          <button class="btn btn--ghost" @click="dismiss">Dismiss</button>
          <button class="btn btn--primary" @click="goToSettings">Go to Settings</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { useHostLiveness } from '~/composables/useHostLiveness';

const router = useRouter();
const { unreachableHosts, checked, checkHosts, dismiss } = useHostLiveness();

const visible = computed(() => checked.value && unreachableHosts.value.length > 0);

onMounted(() => {
  checkHosts();
});

function goToSettings() {
  dismiss();
  router.push('/settings');
}
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: oklch(from var(--gray-9) l c h / 0.5);
  z-index: var(--layer-5, 200);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--size-4);
}

.dialog-panel {
  background: var(--surface-2);
  border-radius: var(--radius-3);
  box-shadow: var(--shadow-4);
  inline-size: 100%;
  max-inline-size: var(--size-md);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-4) var(--size-5);
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.dialog-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-7);
  margin: 0;
  color: var(--red-6);
}

.dialog-close {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-5);
  color: var(--text-2);
  padding: var(--size-1);
  line-height: 1;
}

.dialog-close:hover {
  color: var(--text-1);
}

.dialog-body {
  padding: var(--size-5);
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
}

.dialog-description {
  margin: 0;
  color: var(--text-1);
  font-size: var(--font-size-2);
}

.dialog-hint {
  margin: 0;
  color: var(--text-2);
  font-size: var(--font-size-1);
}

.host-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.host-item {
  padding: var(--size-3) var(--size-4);
  background: var(--surface-1);
  border: var(--border-size-1) solid var(--red-7);
  border-radius: var(--radius-2);
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
}

.host-url {
  font-family: var(--font-mono);
  font-size: var(--font-size-2);
  color: var(--text-1);
  word-break: break-all;
}

.host-error {
  font-size: var(--font-size-1);
  color: var(--red-6);
}

.dialog-footer {
  display: flex;
  gap: var(--size-2);
  justify-content: flex-end;
  padding: var(--size-4) var(--size-5);
  border-block-start: var(--border-size-1) solid var(--surface-3);
}

.btn {
  padding: var(--size-2) var(--size-5);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  border: none;
  font-family: inherit;
}

.btn--primary {
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.btn--ghost {
  background: transparent;
  color: var(--text-2);
}

.btn--ghost:hover {
  color: var(--text-1);
  background: var(--surface-3);
}
</style>
