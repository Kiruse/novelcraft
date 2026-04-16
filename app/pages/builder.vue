<template>
  <div v-if="enabled" class="builder-page">
    <BuilderForm>
      <template #header>
        <h1 class="builder-title">Story Builder</h1>
      </template>
    </BuilderForm>
  </div>
  <div v-else class="disabled">
    <p>Story Builder is not enabled.</p>
  </div>
</template>

<script setup lang="ts">
const { storyBuilder: enabled } = useRuntimeConfig().public;
</script>

<style scoped>
.builder-page {
  max-inline-size: var(--size-content-3);
  margin-inline: auto;
  padding: var(--size-8);
}

.builder-title {
  font-size: var(--font-size-6);
  font-weight: var(--font-weight-7);
  margin-block-end: var(--size-8);
}

.disabled {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}
</style>
