<template>
  <div class="vignettes-page">
    <div class="vignettes-header">
      <h1 class="page-title">Vignettes</h1>
      <p class="page-subtitle">Your quick-play interactive stories</p>
    </div>

    <div v-if="allVignettes.length > 0" class="vignettes-list">
      <NuxtLink
        v-for="v in allVignettes"
        :key="v.id"
        :to="`/vignettes/${v.id}`"
        class="vignette-row"
      >
        <div class="vignette-row-info">
          <span class="vignette-row-title">{{ v.title }}</span>
          <span v-if="v.description" class="vignette-row-disposition">{{ truncate(v.description, 120) }}</span>
        </div>
        <div class="vignette-row-meta">
          <span class="vignette-row-date">{{ formatDate(v.updatedAt) }}</span>
        </div>
      </NuxtLink>
    </div>
    <div v-else class="empty-state">
      <p>No vignettes yet. Start one from the home page!</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { VignetteShape } from '~/composables/useCurrentUser';

const { data } = await useFetch<{ vignettes: VignetteShape[] }>('/api/vignettes');
const allVignettes = computed(() => data.value?.vignettes ?? []);

function formatDate(date: string): string {
  const d = new Date(date);
  const now = new Date();
  const diff = now.getTime() - d.getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7) return `${days} days ago`;
  return d.toLocaleDateString();
}

function truncate(text: string, max: number): string {
  if (text.length <= max) return text;
  return text.slice(0, max) + '…';
}
</script>

<style scoped>
.vignettes-page {
  max-inline-size: var(--size-lg);
  margin-inline: auto;
  padding: var(--size-8);
}

.vignettes-header {
  margin-block-end: var(--size-8);
}

.page-title {
  font-size: var(--font-size-6);
  font-weight: var(--font-weight-7);
  margin-block-end: var(--size-2);
}

.page-subtitle {
  font-size: var(--font-size-2);
  color: var(--text-2);
}

.vignettes-list {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.vignette-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-4) var(--size-5);
  background: var(--surface-1);
  border-radius: var(--radius-3);
  box-shadow: var(--shadow-1);
  text-decoration: none;
  color: inherit;
  transition: background var(--animation-duration, 0.15s) var(--ease-2),
    transform var(--animation-duration, 0.15s) var(--ease-2);
}

.vignette-row:hover {
  background: var(--surface-3);
  transform: translateY(calc(var(--size-1) * -1));
}

.vignette-row-info {
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
  min-inline-size: 0;
  flex: 1;
}

.vignette-row-title {
  font-size: var(--font-size-3);
  font-weight: var(--font-weight-6);
}

.vignette-row-disposition {
  font-size: var(--font-size-1);
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vignette-row-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--size-1);
  flex-shrink: 0;
  margin-inline-start: var(--size-4);
}

.vignette-row-date {
  font-size: var(--font-size-0);
  color: var(--text-2);
}

.empty-state {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}
</style>
