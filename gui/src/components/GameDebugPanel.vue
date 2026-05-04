<template>
  <aside class="debug-panel">
    <div class="debug-header">
      <span class="debug-title">Debug</span>
    </div>

    <div class="debug-body">
      <div v-if="loading" class="debug-loading">Loading…</div>
      <div v-else-if="error" class="debug-error">{{ error }}</div>
      <template v-else>
        <section v-for="(mod, type) in data?.modules" :key="type" class="debug-module">
          <Collapsible v-model="mod._open">
            <template #header>{{ type }}</template>

            <!-- State -->
            <div class="debug-section">
              <span class="debug-label">State</span>
              <pre class="debug-json">{{ formatJson(mod.state) }}</pre>
            </div>

            <!-- Knowledge -->
            <div v-if="mod.knowledge" class="debug-section">
              <span class="debug-label">Knowledge</span>
              <pre class="debug-json">{{ formatJson(mod.knowledge) }}</pre>
            </div>

            <!-- Tools -->
            <div v-if="mod.tools.length > 0" class="debug-section">
              <span class="debug-label">Tools</span>
              <div class="debug-tags">
                <span v-for="t in mod.tools" :key="t" class="debug-tag">{{ t }}</span>
              </div>
            </div>

            <!-- Manual state editor -->
            <div class="debug-section">
              <span class="debug-label">Set state value</span>
              <form class="debug-edit" @submit.prevent="patchState(type, mod)">
                <input v-model="mod._patchPath" type="text" placeholder="path e.g. variables.my_var" class="debug-input" />
                <select v-model="mod._patchType" class="debug-select">
                  <option value="bool">bool</option>
                  <option value="number">number</option>
                  <option value="string">string</option>
                  <option value="json">json</option>
                </select>
                <input v-model="mod._patchValue" type="text" placeholder="value" class="debug-input" />
                <button type="submit" class="debug-patch-btn" :disabled="mod._patching">Set</button>
              </form>
            </div>
          </Collapsible>
        </section>
      </template>
    </div>

    <div class="debug-footer">
      <button type="button" class="debug-refresh" @click="fetchDebug">↻ Refresh</button>
    </div>
  </aside>
</template>

<script setup lang="ts">
import Collapsible from '~/components/Collapsible.vue';

const props = defineProps<{
  sessionId: number;
}>();

const loading = ref(false);
const error = ref<string | null>(null);

interface DebugModule {
  config: unknown;
  state: unknown;
  knowledge: unknown;
  tools: string[];
  _open: boolean;
  _patchPath: string;
  _patchType: 'bool' | 'number' | 'string' | 'json';
  _patchValue: string;
  _patching: boolean;
}

const data = ref<{
  modules: Record<string, DebugModule>;
} | null>(null);

onMounted(() => {
  fetchDebug();
});

async function fetchDebug() {
  loading.value = true;
  error.value = 'Debug panel is temporarily disabled (migrating to local-first architecture)';
  loading.value = false;
}

function formatJson(val: unknown): string {
  try {
    return JSON.stringify(val, null, 2);
  } catch {
    return String(val);
  }
}

async function patchState(moduleId: string, mod: DebugModule) {
  alert('Debug panel is temporarily disabled (migrating to local-first architecture)');
}
</script>

<style scoped>
.debug-panel {
  inline-size: var(--size-content-2);
  block-size: 100%;
  flex-shrink: 0;
  background: var(--surface-2);
  border-inline-start: var(--border-size-1) solid var(--surface-3);
  display: flex;
  flex-direction: column;
  font-size: var(--font-size-0);
}

.debug-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-3) var(--size-4);
  border-block-end: var(--border-size-1) solid var(--surface-3);
  background: var(--surface-2);
}

.debug-title {
  font-weight: var(--font-weight-7);
  font-size: var(--font-size-1);
  text-transform: uppercase;
  letter-spacing: var(--font-letterspacing-3);
  color: var(--text-2);
}

.debug-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--size-3);
}

.debug-loading,
.debug-error {
  padding: var(--size-4);
  text-align: center;
  color: var(--text-2);
}

.debug-error {
  color: var(--red-6);
}

.debug-module {
  margin-block-end: var(--size-3);
  border: var(--border-size-1) solid var(--surface-3);
  border-radius: var(--radius-2);
  overflow: hidden;
}

.debug-section {
  margin-block-end: var(--size-3);
}

.debug-label {
  display: block;
  font-weight: var(--font-weight-6);
  color: var(--text-2);
  margin-block-end: var(--size-1);
  font-size: var(--font-size-0);
  text-transform: uppercase;
  letter-spacing: var(--font-letterspacing-2);
}

.debug-json {
  background: var(--surface-2);
  border-radius: var(--radius-1);
  padding: var(--size-2);
  font-family: var(--font-mono, monospace);
  font-size: var(--font-size-0);
  line-height: var(--font-lineheight-4);
  overflow-x: auto;
  white-space: pre-wrap;
  margin: 0;
  max-block-size: 12lh;
  overflow-y: auto;
}

.debug-tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--size-1);
}

.debug-tag {
  background: var(--surface-2);
  border-radius: var(--radius-1);
  padding: var(--size-1) var(--size-2);
  font-size: var(--font-size-0);
  font-family: var(--font-mono, monospace);
  color: var(--text-2);
}

.debug-edit {
  display: flex;
  gap: var(--size-1);
  align-items: center;
}

.debug-input {
  flex: 1;
  min-inline-size: 0;
  font-size: var(--font-size-0);
  padding: var(--size-1) var(--size-2);
  border-radius: var(--radius-1);
}

.debug-select {
  font-size: var(--font-size-0);
  padding: var(--size-1);
  border-radius: var(--radius-1);
}

.debug-patch-btn {
  padding: var(--size-1) var(--size-3);
  background: var(--indigo-9);
  color: var(--indigo-2);
  border: none;
  border-radius: var(--radius-1);
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-5);
  cursor: pointer;
  white-space: nowrap;
}

.debug-patch-btn:hover:not(:disabled) {
  background: var(--indigo-8);
}

.debug-patch-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.debug-footer {
  padding: var(--size-3) var(--size-4);
  border-block-start: var(--border-size-1) solid var(--surface-3);
}

.debug-refresh {
  inline-size: 100%;
  padding: var(--size-2);
  background: var(--surface-3);
  border: none;
  border-radius: var(--radius-2);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  cursor: pointer;
  color: var(--text-1);
}

.debug-refresh:hover {
  background: var(--surface-4);
}
</style>
