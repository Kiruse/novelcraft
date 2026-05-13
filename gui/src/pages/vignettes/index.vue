<template>
  <div class="vignettes-page">
    <div class="vignettes-header">
      <h1 class="page-title">Vignettes</h1>
      <p class="page-subtitle">Your quick-play interactive stories</p>
    </div>

    <div class="vignettes-list">
      <RouterLink
        ref="newVignetteRef"
        to="/vignettes/new"
        class="vignette-row vignette-row--new"
        :class="{ 'vignette-row--focused': focusedIndex === 0 }"
        @click.prevent="createVignette"
        @keydown.enter.prevent="createVignette"
      >
        <div class="vignette-row-info">
          <span class="vignette-row-title">+ New vignette</span>
          <span class="vignette-row-disposition">Start a quick-play story</span>
        </div>
      </RouterLink>

      <template v-if="allVignettes?.length">
        <div
          v-for="(v, i) in allVignettes"
          :key="v.id"
          class="vignette-row-wrapper"
        >
          <RouterLink
            :ref="el => { if (el) rowRefs[i] = el }"
            :to="`/vignettes/${v.id}`"
            class="vignette-row"
            :class="{ 'vignette-row--focused': i + 1 === focusedIndex }"
            @keydown.enter.prevent="router.push(`/vignettes/${v.id}`)"
          >
            <div class="vignette-row-info">
              <span class="vignette-row-title">{{ v.title }}</span>
              <span v-if="v.description" class="vignette-row-disposition">{{ truncate(v.description, 120) }}</span>
            </div>
            <div class="vignette-row-meta">
              <span class="vignette-row-date">{{ formatDate(v.updatedAt) }}</span>
              <button
                class="vignette-delete-btn"
                aria-label="Delete vignette"
                @click.prevent="pendingDeleteId = v.id"
              >&times;</button>
            </div>
          </RouterLink>
          <div v-if="pendingDeleteId === v.id" class="confirm-overlay" @click.self="pendingDeleteId = null">
            <div class="confirm-box">
              <p class="confirm-text">Delete this vignette?</p>
              <div class="confirm-actions">
                <button class="btn btn--danger" @click="confirmDelete">Delete</button>
                <button class="btn btn--ghost" @click="pendingDeleteId = null">Cancel</button>
              </div>
            </div>
          </div>
        </div>
      </template>
      <div v-else class="empty-state">
        <p>No vignettes yet. Create one above!</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useToast } from '~/composables/useToast';
import { useVignettes } from '~/composables/useVignettes';
import { getErrorDisplay } from '~/utils/msgUtils';

const router = useRouter();
const {
  vignettes: allVignettes,
  loadVignettes,
  create,
  remove,
} = useVignettes();
const toast = useToast();

const focusedIndex = ref(-1);
const rowRefs = ref<unknown[]>([]);
const newVignetteRef = ref<unknown>(null);
const pendingDeleteId = ref<string | null>(null);

const totalRows = computed(() => 1 + (allVignettes.value?.length ?? 0));

function resolveEl(raw: unknown): HTMLElement | null {
  if (!raw) return null;
  if (raw instanceof HTMLElement) return raw;
  if (typeof raw === 'object' && raw !== null && '$el' in raw) return (raw as { $el: HTMLElement }).$el;
  return null;
}

function moveFocus(delta: number) {
  if (totalRows.value === 0) return;
  focusedIndex.value = Math.max(0, Math.min(totalRows.value - 1, focusedIndex.value + delta));
  if (focusedIndex.value === 0) {
    resolveEl(newVignetteRef.value)?.focus();
  } else {
    resolveEl(rowRefs.value[focusedIndex.value - 1])?.focus();
  }
}

async function createVignette() {
  const id = await create();
  router.push(`/vignettes/${id}`);
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

async function confirmDelete() {
  if (!pendingDeleteId.value) return;
  const id = pendingDeleteId.value;
  pendingDeleteId.value = null;
  try {
    await remove(id);
  } catch (err) {
    toast.error('Failed to delete vignette: ' + getErrorDisplay(err));
    console.error('Vignette deletion failed:', err);
  }
}

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

onMounted(async () => {
  document.addEventListener('keydown', onListKeydown);
  await loadVignettes();
});

onUnmounted(() => {
  document.removeEventListener('keydown', onListKeydown);
});
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

.vignette-row-wrapper {
  position: relative;
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

.vignette-delete-btn {
  background: none;
  border: none;
  font-size: var(--font-size-4);
  color: var(--text-3);
  cursor: pointer;
  padding: 0 var(--size-2);
  line-height: 1;
  opacity: 0;
  transition: opacity var(--animation-duration, 0.15s) var(--ease-2),
    color var(--animation-duration, 0.15s) var(--ease-2);
}

.vignette-row:hover .vignette-delete-btn,
.vignette-row--focused .vignette-delete-btn {
  opacity: 1;
}

.vignette-delete-btn:hover {
  color: var(--red-6);
}

.confirm-overlay {
  position: absolute;
  inset: 0;
  background: oklch(from var(--gray-9) l c h / 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1;
  border-radius: var(--radius-3);
}

.confirm-box {
  background: var(--surface-2);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-3);
  padding: var(--size-5);
  box-shadow: var(--shadow-3);
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
  min-inline-size: 16rem;
}

.confirm-text {
  font-size: var(--font-size-2);
  color: var(--text-1);
  margin: 0;
}

.confirm-actions {
  display: flex;
  gap: var(--size-2);
  justify-content: flex-end;
}

.btn {
  padding: var(--size-3) var(--size-8);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  border: none;
}

.btn--danger {
  background: var(--red-6);
  color: var(--gray-0);
}

.btn--danger:hover {
  opacity: 0.9;
}

.btn--ghost {
  background: transparent;
  color: var(--text-2);
}

.btn--ghost:hover {
  color: var(--text-1);
  background: var(--surface-3);
}

.empty-state {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}
</style>
