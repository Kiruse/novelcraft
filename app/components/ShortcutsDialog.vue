<template>
  <Teleport to="body">
    <div v-if="open" class="dialog-overlay" @click.self="close">
      <div class="dialog-panel">
        <div class="dialog-header">
          <h2 class="dialog-title">Keyboard shortcuts</h2>
          <button class="dialog-close" @click="close">&times;</button>
        </div>

        <div class="dialog-toolbar">
          <input
            ref="searchRef"
            v-model="query"
            class="dialog-search"
            placeholder="Search shortcuts…"
            @keydown.escape.prevent="close"
          />
          <label class="filter-toggle">
            <input type="checkbox" v-model="filterByKeys" class="filter-toggle-input" />
            <span class="filter-toggle-label">By keys</span>
          </label>
        </div>

        <div class="dialog-body">
          <template v-for="group in filteredGroups" :key="group.title">
            <h3 class="shortcuts-heading">{{ group.title }}</h3>
            <div class="shortcuts-list">
              <ShortcutRow
                v-for="(s, i) in group.shortcuts"
                :key="i"
                :keys="s.keys"
                :label="s.label"
                :highlight="query || filterByKeys ? matchText(s, query) : ''"
              />
            </div>
          </template>
          <p v-if="noResults" class="no-results">No shortcuts match "{{ query }}"</p>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import ShortcutRow from '~/components/ShortcutRow.vue';

interface Shortcut {
  keys: string;
  label: string;
}

interface ShortcutGroup {
  title: string;
  shortcuts: Shortcut[];
}

const SHORTCUTS: ShortcutGroup[] = [
  {
    title: 'Global',
    shortcuts: [
      { keys: 'Ctrl Shift /', label: 'Open shortcuts dialog' },
      { keys: 'Alt N → H', label: 'Go to Home' },
      { keys: 'Alt N → V', label: 'Go to Vignettes' },
    ],
  },
  {
    title: 'Vignettes list',
    shortcuts: [
      { keys: 'j | ↓', label: 'Select next vignette' },
      { keys: 'k | ↑', label: 'Select previous vignette' },
      { keys: 'Enter', label: 'Open selected vignette' },
    ],
  },
  {
    title: 'Game — navigation',
    shortcuts: [
      { keys: 'Alt ←', label: 'Previous page' },
      { keys: 'Alt →', label: 'Next page' },
      { keys: 'PageUp | PageDown', label: 'Scroll the prose area' },
    ],
  },
  {
    title: 'Game — input focus',
    shortcuts: [
      { keys: 'Alt ↑', label: 'Focus previous slot (chat → prose → prompt → system)' },
      { keys: 'Alt ↓', label: 'Focus next slot (system → prompt → prose → chat)' },
      { keys: 'Ctrl Enter', label: 'Save any active editor & focus chat bar' },
    ],
  },
  {
    title: 'Game — chat bar',
    shortcuts: [
      { keys: 'Shift Tab', label: 'Cycle input mode (Write → Steer → Instruct)' },
      { keys: 'Enter', label: 'Send message' },
      { keys: '↑ (empty input)', label: 'Edit last response' },
    ],
  },
  {
    title: 'Game — editors',
    shortcuts: [
      { keys: 'Escape', label: 'Close editor without saving' },
      { keys: 'Ctrl Enter', label: 'Save & return to chat bar' },
      { keys: 'Ctrl ↑ | Ctrl ↓', label: 'Jump between paragraphs (prose editor)' },
    ],
  },
  {
    title: 'Game — slash commands',
    shortcuts: [
      { keys: '/write', label: 'Switch to Write mode' },
      { keys: '/steer', label: 'Switch to Steer / Remind mode' },
      { keys: '/instruct', label: 'Switch to Instruct mode' },
    ],
  },
];

const { open, close } = useShortcutsDialog();

const query = ref('');
const filterByKeys = ref(false);
const searchRef = ref<HTMLInputElement | null>(null);

watch(open, (v) => {
  if (v) {
    query.value = '';
    filterByKeys.value = false;
    nextTick(() => searchRef.value?.focus());
  }
});

function matchText(shortcut: Shortcut, q: string): string {
  if (filterByKeys.value) return shortcut.keys.toLowerCase().includes(q.toLowerCase()) ? 'keys' : '';
  return shortcut.label.toLowerCase().includes(q.toLowerCase()) ? 'label' : '';
}

const filteredGroups = computed(() => {
  if (!query.value) return SHORTCUTS;
  const q = query.value.toLowerCase();
  return SHORTCUTS
    .map((group) => ({
      ...group,
      shortcuts: group.shortcuts.filter((s) => {
        if (filterByKeys.value) return s.keys.toLowerCase().includes(q);
        return s.label.toLowerCase().includes(q);
      }),
    }))
    .filter((g) => g.shortcuts.length > 0);
});

const noResults = computed(() => query.value && filteredGroups.value.length === 0);
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: oklch(from var(--gray-9) l c h / 0.5);
  z-index: var(--layer-5, 200);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--size-4);
}

.dialog-panel {
  background: var(--surface-2);
  border-radius: var(--radius-3);
  box-shadow: var(--shadow-4);
  inline-size: 100%;
  max-inline-size: var(--size-lg);
  max-block-size: 80dvh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-4) var(--size-5);
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.dialog-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-7);
  margin: 0;
}

.dialog-close {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-5);
  color: var(--text-2);
  padding: var(--size-1);
  line-height: 1;
}

.dialog-close:hover {
  color: var(--text-1);
}

.dialog-toolbar {
  display: flex;
  align-items: center;
  gap: var(--size-3);
  padding: var(--size-3) var(--size-5);
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.dialog-search {
  flex: 1;
  padding: var(--size-2) var(--size-3);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  background: var(--surface-1);
  color: var(--text-1);
  font-size: var(--font-size-2);
  font-family: inherit;
}

.dialog-search:focus {
  outline: none;
  border-color: var(--indigo-6);
}

.dialog-search::placeholder {
  color: var(--text-3);
}

.filter-toggle {
  display: flex;
  align-items: center;
  gap: var(--size-2);
  cursor: pointer;
  flex-shrink: 0;
  user-select: none;
}

.filter-toggle-input {
  appearance: none;
  inline-size: var(--size-5);
  block-size: var(--size-5);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-1);
  background: var(--surface-1);
  cursor: pointer;
  position: relative;
  transition: background var(--animation-duration, 0.15s) var(--ease-2),
    border-color var(--animation-duration, 0.15s) var(--ease-2);
}

.filter-toggle-input:checked {
  background: var(--indigo-6);
  border-color: var(--indigo-6);
}

.filter-toggle-input:checked::after {
  content: '';
  position: absolute;
  inset-block-start: 1px;
  inset-inline-start: 4px;
  inline-size: 4px;
  block-size: 8px;
  border: solid var(--gray-0);
  border-width: 0 2px 2px 0;
  transform: rotate(45deg);
}

.filter-toggle-label {
  font-size: var(--font-size-1);
  color: var(--text-2);
  font-weight: var(--font-weight-5);
}

.dialog-body {
  padding: var(--size-4) var(--size-5);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
}

.shortcuts-heading {
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-6);
  color: var(--text-2);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: 0;
  padding-block-end: var(--size-1);
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.shortcuts-list {
  display: flex;
  flex-direction: column;
}

.no-results {
  text-align: center;
  color: var(--text-3);
  font-size: var(--font-size-2);
  font-style: italic;
  padding: var(--size-6) 0;
}
</style>
