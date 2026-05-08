<template>
  <div class="collapsible">
    <button
      class="collapsible-header"
      :aria-expanded="isOpen"
      @click.prevent="isOpen = !isOpen"
    >
      <Chevron :open="isOpen" />
      <span class="collapsible-label"><slot name="header" /></span>
      <slot name="header-actions" />
    </button>
    <div ref="bodyRef" class="collapsible-body" :class="{ 'collapsible-body--open': isOpen }">
      <div class="collapsible-body-inner">
        <slot />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import Chevron from '~/components/Chevron.vue';

const props = withDefaults(defineProps<{ defaultExpanded?: boolean }>(), {
  defaultExpanded: false,
});

const modelValue = defineModel<boolean | undefined>();
const isOpen = computed({
  get: () => modelValue.value ?? props.defaultExpanded,
  set: (value: boolean) => modelValue.value = value,
});

const bodyRef = ref<HTMLElement | null>(null);

// Animate height on toggle using Web Animations API
watch(isOpen, async (open) => {
  const el = bodyRef.value;
  if (!el) return;

  const inner = el.querySelector('.collapsible-body-inner') as HTMLElement;
  if (!inner) return;

  // Cancel any in-flight animation
  el.getAnimations().forEach((a) => a.cancel());

  const targetHeight = inner.scrollHeight;

  if (open) {
    // Expand: 0 → scrollHeight
    el.style.height = '0px';
    const anim = el.animate(
      [{ height: '0px' }, { height: `${targetHeight}px` }],
      { duration: 200, easing: 'cubic-bezier(0.33, 1, 0.68, 1)' },
    );
    await anim.finished;
    el.style.height = '';
  } else {
    // Collapse: current → 0
    el.style.height = `${el.scrollHeight}px`;
    // Force reflow so the browser registers the start height
    el.offsetHeight;
    const anim = el.animate(
      [{ height: `${el.scrollHeight}px` }, { height: '0px' }],
      { duration: 200, easing: 'cubic-bezier(0.33, 1, 0.68, 1)' },
    );
    await anim.finished;
    el.style.height = '0px';
  }
});

// Ensure correct initial state
onMounted(() => {
  if (!isOpen.value && bodyRef.value) {
    bodyRef.value.style.height = '0px';
  }
});
</script>

<style scoped>
.collapsible {
  inline-size: 100%;
}

.collapsible-header {
  display: flex;
  align-items: center;
  gap: var(--size-2);
  inline-size: 100%;
  background: none;
  border: none;
  padding: var(--collapsible-header-padding, var(--size-3) var(--size-4));
  cursor: pointer;
  color: inherit;
  font: inherit;
  text-align: start;
}

.collapsible-label {
  flex: 1;
  font-weight: var(--font-weight-6);
  color: var(--text-1);
}

.collapsible-body {
  overflow: hidden;
}

.collapsible-body-inner {
  padding: var(--collapsible-body-padding, 0 var(--size-4) var(--size-3));
}
</style>
