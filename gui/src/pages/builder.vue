<template>
  <div class="builder-page">
    <BuilderForm>
      <template #header>
        <h1 class="builder-title">Story Builder</h1>
      </template>
    </BuilderForm>
  </div>
</template>

<script setup lang="ts">
import BuilderForm from '~/components/builder/BuilderForm.vue';
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
</style>
