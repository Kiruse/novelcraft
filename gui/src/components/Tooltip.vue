<template>
  <Teleport to="body">
    <div
      v-if="visible"
      ref="floatingRef"
      class="tooltip"
      :style="positionStyle"
      role="tooltip"
    >
      <slot />
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computePosition, offset as offsetMiddleware, shift as shiftMiddleware, autoUpdate } from '@floating-ui/dom';

const props = withDefaults(
  defineProps<{
    anchor: () => HTMLElement | null | undefined;
    visible?: boolean;
  }>(),
  { visible: true },
);

const floatingRef = ref<HTMLElement | null>(null);
const positionStyle = ref<Record<string, string>>({ position: 'absolute', left: '0', top: '0' });

let cleanup: (() => void) | null = null;

async function updatePosition() {
  const anchor = props.anchor();
  const floating = floatingRef.value;
  if (!anchor || !floating) return;

  const { x, y } = await computePosition(anchor, floating, {
    placement: 'bottom-start',
    middleware: [offsetMiddleware(4), shiftMiddleware({ padding: 4 })],
  });

  positionStyle.value = { position: 'absolute', left: `${x}px`, top: `${y}px` };
}

watch(
  [() => props.visible],
  (visible) => {
    if (visible) {
      nextTick(() => {
        const anchor = props.anchor();
        const floating = floatingRef.value;
        if (!anchor || !floating) return;
        cleanup = autoUpdate(anchor, floating, updatePosition);
      });
    } else {
      cleanup?.();
      cleanup = null;
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  cleanup?.();
  cleanup = null;
});
</script>

<style scoped>
.tooltip {
  padding: var(--size-1) var(--size-2);
  font-size: var(--font-size-0);
  color: var(--gray-0);
  background: var(--red-7);
  border-radius: var(--radius-1);
  white-space: nowrap;
  pointer-events: none;
  z-index: var(--layer-4, 100);
}
</style>
