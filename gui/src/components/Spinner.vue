<template>
  <span class="spinner" :class="`spinner--${size}`" :aria-label="label" role="status"></span>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    size?: 'sm' | 'md';
    label?: string;
  }>(),
  {
    size: 'sm',
    label: 'Loading',
  },
);
</script>

<style scoped>
.spinner {
  display: inline-block;
  border: var(--border-size-2) solid var(--surface-4);
  border-top-color: var(--indigo-5);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

.spinner--sm {
  inline-size: var(--size-4);
  block-size: var(--size-4);
}

.spinner--md {
  inline-size: var(--size-6);
  block-size: var(--size-6);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
