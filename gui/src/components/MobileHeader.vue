<template>
  <header class="mobile-header">
    <RouterLink to="/" class="mobile-logo">
      <span class="logo-mark">◆</span>
      <span class="logo-text">NovelCraft</span>
    </RouterLink>
    <button class="hamburger" aria-label="Open menu" @click="$emit('open')">
      <span class="hamburger-bar" />
      <span class="hamburger-bar" />
      <span class="hamburger-bar" />
    </button>
  </header>
</template>

<script setup lang="ts">
defineEmits<{ open: [] }>();
</script>

<style scoped>
.mobile-header {
  display: none;
}

@media (max-width: 767px) {
  .mobile-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--size-3) var(--size-4);
    background: var(--surface-1);
    border-block-end: var(--border-size-1) solid var(--surface-3);
  }

  .mobile-logo {
    display: flex;
    align-items: center;
    gap: var(--size-2);
    text-decoration: none;
    color: var(--text-1);
  }

  .logo-mark {
    font-size: var(--font-size-3);
  }

  .logo-text {
    font-size: var(--font-size-3);
    font-weight: var(--font-weight-8);
    letter-spacing: -0.02em;
  }

  .hamburger {
    display: flex;
    flex-direction: column;
    gap: var(--size-1);
    background: none;
    border: none;
    cursor: pointer;
    padding: var(--size-2);
  }

  .hamburger-bar {
    inline-size: var(--size-5);
    block-size: var(--border-size-2);
    background: var(--text-1);
    border-radius: var(--radius-round);
  }
}
</style>
