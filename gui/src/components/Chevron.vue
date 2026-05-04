<template>
  <span class="chevron" :style="{ transform: currentRotation }">
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="1em"
      height="1em"
      viewBox="0 0 16 16"
      fill="currentColor"
    >
      <path d="M6 3.5L10.5 8L6 12.5L6.7 13.2L11.9 8L6.7 2.8Z" />
    </svg>
  </span>
</template>

<script setup lang="ts">
/**
 * A small directional chevron arrow.
 * - direction="right" (default): closed = right ▸, open = down ▾
 * - direction="down":              closed = down ▾, open = up ▴
 */
const props = withDefaults(defineProps<{
  open?: boolean;
  direction?: 'right' | 'down';
}>(), {
  open: false,
  direction: 'right',
});

const currentRotation = computed(() => {
  const base = props.direction === 'down' ? 90 : 0;
  const extra = props.open ? 90 : 0;
  return `rotate(${base + extra}deg)`;
});
</script>

<style scoped>
.chevron {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--text-2);
  font-size: var(--font-size-1);
  line-height: 1;
  transition: transform var(--animation-duration, 0.15s) var(--ease-2);
}
</style>
