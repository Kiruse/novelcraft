<template>
  <Teleport to="body">
    <div class="toast-container">
      <TransitionGroup name="toast-list">
        <ToastItem
          v-for="toast in toasts"
          :key="toast.id"
          :toast="toast"
          @dismiss="dismiss"
        />
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { useToast } from '~/composables/useToast';
import ToastItem from '~/components/ToastItem.vue';

const { toasts, dismiss } = useToast();
</script>

<style scoped>
.toast-container {
  position: fixed;
  inset-block-end: var(--size-5);
  inset-inline-end: var(--size-5);
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
  pointer-events: none;
}

.toast-container > * {
  pointer-events: auto;
}

/* TransitionGroup classes */
.toast-list-enter-active {
  transition: all 0.25s ease-out;
}
.toast-list-leave-active {
  transition: all 0.2s ease-in;
}
.toast-list-enter-from {
  opacity: 0;
  transform: translateX(100%);
}
.toast-list-leave-to {
  opacity: 0;
  transform: translateX(50%);
}
.toast-list-move {
  transition: transform 0.25s ease;
}
</style>
