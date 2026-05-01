<template>
  <div class="vignettes-page">
    <div class="vignettes-header">
      <h1 class="page-title">Vignettes</h1>
      <p class="page-subtitle">Your quick-play interactive stories</p>
    </div>

    <div class="vignettes-list">
      <NuxtLink
        ref="newVignetteRef"
        to="/vignettes/new"
        class="vignette-row vignette-row--new"
        :class="{ 'vignette-row--focused': focusedIndex === 0 }"
        @keydown.enter.prevent="navigateTo('/vignettes/new')"
      >
        <div class="vignette-row-info">
          <span class="vignette-row-title">+ New vignette</span>
          <span class="vignette-row-disposition">Start a quick-play story</span>
        </div>
      </NuxtLink>

      <template v-if="allVignettes.length > 0">
        <NuxtLink
          v-for="(v, i) in allVignettes"
          :key="v.id"
          :ref="el => { if (el) rowRefs[i] = el }"
          :to="`/vignettes/${v.id}`"
          class="vignette-row"
          :class="{ 'vignette-row--focused': i + 1 === focusedIndex }"
          @keydown.enter.prevent="navigateTo(`/vignettes/${v.id}`)"
        >
          <div class="vignette-row-info">
            <span class="vignette-row-title">{{ v.title }}</span>
            <span v-if="v.description" class="vignette-row-disposition">{{ truncate(v.description, 120) }}</span>
          </div>
          <div class="vignette-row-meta">
            <span class="vignette-row-date">{{ formatDate(v.updatedAt) }}</span>
          </div>
        </NuxtLink>
      </template>
      <div v-else class="empty-state">
        <p>No vignettes yet. Create one above!</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const db = useLocalDb();
const { localSessions } = await import('#shared/db/localSchema');
const { desc } = await import('drizzle-orm');

const allVignettes = ref<Array<{
  id: string;
  title: string;
  description: string | null;
  updatedAt: string;
}>>([]);

function resolveEl(raw: unknown): HTMLElement | null {
  if (!raw) return null;
  if (raw instanceof HTMLElement) return raw;
  if (typeof raw === 'object' && raw !== null && '$el' in raw) return (raw as { $el: HTMLElement }).$el;
  return null;
}

const focusedIndex = ref(-1);
const rowRefs = ref<unknown[]>([]);
const newVignetteRef = ref<unknown>(null);

const totalRows = computed(() => 1 + allVignettes.value.length);

function moveFocus(delta: number) {
  if (totalRows.value === 0) return;
  focusedIndex.value = Math.max(0, Math.min(totalRows.value - 1, focusedIndex.value + delta));
  if (focusedIndex.value === 0) {
    resolveEl(newVignetteRef.value)?.focus();
  } else {
    resolveEl(rowRefs.value[focusedIndex.value - 1])?.focus();
  }
}

function onListKeydown(e: KeyboardEvent) {
  if (e.key === 'j' || e.key === 'ArrowDown') {
    e.preventDefault();
    moveFocus(1);
  } else if (e.key === 'k' || e.key === 'ArrowUp') {
    e.preventDefault();
    moveFocus(-1);
  }
}

onMounted(async () => {
  document.addEventListener('keydown', onListKeydown);
  const rows = await db.select().from(localSessions).orderBy(desc(localSessions.updatedAt)).all();
  allVignettes.value = rows.map(r => ({
    id: r.id,
    title: r.title,
    description: r.description,
    updatedAt: r.updatedAt,
  }));
});

onUnmounted(() => {
  document.removeEventListener('keydown', onListKeydown);
});

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

.vignette-row:hover,
.vignette-row--focused {
  background: var(--surface-3);
  transform: translateY(calc(var(--size-1) * -1));
}

.vignette-row:focus {
  outline: 2px solid var(--indigo-5);
  outline-offset: 2px;
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

.vignette-row--new .vignette-row-title {
  color: var(--indigo-6);
}

.vignette-row--new .vignette-row-disposition {
  color: var(--text-3);
  font-style: italic;
}

.empty-state {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}
</style>
