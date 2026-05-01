<template>
  <div class="shortcut-row">
    <span class="shortcut-label" :class="{ 'shortcut-label--muted': highlight && highlight !== 'label' }">
      {{ label }}
    </span>
    <span class="shortcut-keys">
      <kbd
        v-for="(k, i) in keyList"
        :key="i"
        class="shortcut-key"
        :class="{ 'shortcut-key--highlight': highlight === 'keys' }"
      >{{ k }}</kbd>
    </span>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  keys: string;
  label: string;
  highlight?: string;
}>();

const keyList = computed(() => props.keys.split(' '));
</script>

<style scoped>
.shortcut-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-2) 0;
  gap: var(--size-4);
}

.shortcut-label {
  font-size: var(--font-size-2);
  color: var(--text-1);
  transition: opacity var(--animation-duration, 0.15s) var(--ease-2);
}

.shortcut-label--muted {
  opacity: 0.4;
}

.shortcut-keys {
  display: flex;
  gap: var(--size-1);
  flex-shrink: 0;
}

.shortcut-key {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: var(--size-1) var(--size-2);
  background: var(--surface-2);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-1);
  font-family: var(--font-mono, monospace);
  font-size: var(--font-size-1);
  color: var(--text-2);
  min-inline-size: var(--size-6);
  transition: border-color var(--animation-duration, 0.15s) var(--ease-2),
    color var(--animation-duration, 0.15s) var(--ease-2);
}

.shortcut-key--highlight {
  border-color: var(--indigo-5);
  color: var(--indigo-6);
}
</style>
