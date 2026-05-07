<template>
  <div class="settings-page">
    <h1 class="settings-title">Settings</h1>

    <ModelsConfigurator />

    <section class="settings-section settings-section--debug">
      <h2 class="section-title">Debug Zone</h2>
      <button class="btn btn--ghost" @click="resetOnboarding">Reset Onboarding</button>
    </section>
  </div>
</template>

<script setup lang="ts">
import ModelsConfigurator from '~/components/ModelsConfigurator.vue';
import { useOnboarding } from '~/composables/useOnboarding';

const { reset: resetOnboarding } = useOnboarding();
</script>

<style scoped>
.settings-page {
  max-inline-size: var(--size-content-3);
  margin-inline: auto;
  padding: var(--size-8);
}

.settings-title {
  font-size: var(--font-size-6);
  font-weight: var(--font-weight-7);
  margin-block-end: var(--size-8);
}

.settings-section {
  margin-block-start: var(--size-8);
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
}

.settings-section--debug {
  padding-block-start: var(--size-6);
  border-block-start: var(--border-size-1) solid var(--surface-3);
}

.section-title {
  font-size: var(--font-size-3);
  font-weight: var(--font-weight-6);
  margin: 0;
}

.btn {
  padding: var(--size-2) var(--size-5);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  border: none;
  font-family: inherit;
  align-self: flex-start;
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
