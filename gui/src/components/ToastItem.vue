<template>
  <div :class="['toast', `toast--${toast.type}`]" @click="handleDismiss">
    <div class="toast-icon">
      <span>{{ icon }}</span>
    </div>
    <p class="toast-message">{{ toast.message }}</p>
    <button class="toast-close" aria-label="Dismiss" @click.stop="handleDismiss">&times;</button>
    <div v-if="toast.duration !== false" class="toast-progress">
      <div class="toast-progress-bar" :style="progressStyle" />
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Toast } from '~/composables/useToast';

const props = defineProps<{ toast: Toast }>();
const emit = defineEmits<{ dismiss: [id: number] }>();

const elapsed = ref(0);
let rafId: number | null = null;
let startTime = 0;

const icons: Record<Toast['type'], string> = {
  success: '✓',
  info: 'ℹ',
  warning: '⚠',
  error: '✕',
};

const icon = computed(() => icons[props.toast.type]);

const progressStyle = computed(() => {
  if (props.toast.duration === false) return { transform: 'scaleX(1)' };
  const pct = Math.max(0, 1 - elapsed.value / props.toast.duration);
  return { transform: `scaleX(${pct})` };
});

function handleDismiss() {
  emit('dismiss', props.toast.id);
}

function tick() {
  elapsed.value = Date.now() - startTime;
  if (props.toast.duration !== false && elapsed.value >= props.toast.duration) {
    handleDismiss();
    return;
  }
  rafId = requestAnimationFrame(tick);
}

onMounted(() => {
  if (props.toast.duration !== false) {
    startTime = props.toast.createdAt;
    rafId = requestAnimationFrame(tick);
  }
});

onUnmounted(() => {
  if (rafId !== null) cancelAnimationFrame(rafId);
});
</script>

<style scoped>
.toast {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--size-3);
  padding: var(--size-3) var(--size-4);
  padding-inline-end: var(--size-8);
  border-radius: var(--radius-3);
  background: var(--surface-2);
  border: 1px solid var(--toast-border);
  box-shadow: var(--shadow-3);
  color: var(--toast-fg);
  cursor: pointer;
  overflow: hidden;
  animation: toast-enter 0.25s ease-out;
  min-inline-size: 20rem;
  max-inline-size: 32rem;
}

.toast:hover {
  background: var(--surface-3);
}

/* Per-type colors */
.toast--success { --toast-fg: var(--green-9); --toast-border: var(--green-4); --toast-bar: var(--green-5); }
.toast--info    { --toast-fg: var(--blue-9);  --toast-border: var(--blue-4);  --toast-bar: var(--blue-5); }
.toast--warning { --toast-fg: var(--orange-9);--toast-border: var(--orange-4);--toast-bar: var(--orange-5); }
.toast--error   { --toast-fg: var(--red-9);   --toast-border: var(--red-4);   --toast-bar: var(--red-5); }

.toast-icon {
  flex-shrink: 0;
  display: grid;
  place-items: center;
  inline-size: var(--size-7);
  block-size: var(--size-7);
  border-radius: var(--radius-round);
  font-weight: var(--font-weight-7);
  font-size: var(--font-size-3);
}

.toast-message {
  flex: 1;
  margin: 0;
  font-size: var(--font-size-3);
  line-height: var(--font-lineheight-4);
}

.toast-close {
  position: absolute;
  inset-block-start: var(--size-2);
  inset-inline-end: var(--size-2);
  background: none;
  border: none;
  color: var(--text-2);
  font-size: var(--font-size-4);
  line-height: 1;
  cursor: pointer;
  padding: var(--size-1);
  border-radius: var(--radius-1);
}

.toast-close:hover {
  color: var(--text-1);
  background: var(--surface-3);
}

.toast-progress {
  position: absolute;
  inset-block-end: 0;
  inset-inline: 0;
  block-size: 3px;
  background: color-mix(in oklch, var(--toast-bar) 30%, transparent);
}

.toast-progress-bar {
  block-size: 100%;
  inline-size: 100%;
  background: var(--toast-bar);
  transform-origin: inline-start;
  will-change: transform;
}

@keyframes toast-enter {
  from {
    opacity: 0;
    transform: translateY(var(--size-4));
  }
}
</style>
