<template>
  <aside v-if="open" class="debug-panel">
    <div class="debug-header">
      <span class="debug-title">Debug</span>
      <button type="button" class="debug-close" @click="$emit('update:open', false)">✕</button>
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
const props = defineProps<{
  open: boolean;
  sessionId: number;
}>();

defineEmits<{ 'update:open': [value: boolean] }>();

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

watch(() => props.open, (isOpen) => {
  if (isOpen && !data.value) fetchDebug();
});

async function fetchDebug() {
  loading.value = true;
  error.value = null;
  try {
    const res = await $fetch<{
      sessionId: number;
      modules: Record<string, {
        config: unknown;
        state: unknown;
        knowledge: unknown;
        tools: string[];
      }>;
    }>(`/api/sessions/${props.sessionId}/debug`);

    // Add UI-only fields
    const modules: Record<string, DebugModule> = {};
    for (const [type, mod] of Object.entries(res.modules)) {
      modules[type] = {
        ...mod,
        _open: true,
        _patchPath: '',
        _patchType: 'bool',
        _patchValue: '',
        _patching: false,
      };
    }
    data.value = { modules };
  } catch (e: any) {
    error.value = e?.data?.statusMessage ?? e?.message ?? 'Failed to load debug data';
  } finally {
    loading.value = false;
  }
}

function formatJson(val: unknown): string {
  try {
    return JSON.stringify(val, null, 2);
  } catch {
    return String(val);
  }
}

async function patchState(moduleId: string, mod: DebugModule) {
  const path = mod._patchPath.trim();
  if (!path) return;

  let value: unknown;
  switch (mod._patchType) {
    case 'bool': value = mod._patchValue === 'true'; break;
    case 'number': value = Number(mod._patchValue); break;
    case 'string': value = mod._patchValue; break;
    case 'json':
      try { value = JSON.parse(mod._patchValue); } catch {
        alert('Invalid JSON'); return;
      }
      break;
  }

  // Build nested patch from dot-path: "a.b.c" → { a: { b: { c: value } } }
  const patch: Record<string, unknown> = {};
  const keys = path.split('.');
  let current: Record<string, unknown> = patch;
  for (let i = 0; i < keys.length; i++) {
    const key = keys[i]!;
    if (i === keys.length - 1) {
      current[key] = value;
    } else {
      current[key] = {};
      current = current[key] as Record<string, unknown>;
    }
  }

  mod._patching = true;
  try {
    const res = await $fetch<{
      moduleId: string;
      state: Record<string, unknown>;
    }>(`/api/sessions/${props.sessionId}/debug`, {
      method: 'PATCH',
      body: { moduleId, patch },
    });
    mod.state = res.state;
    // Refresh knowledge
    await fetchDebug();
  } catch (e: any) {
    alert(e?.data?.statusMessage ?? e?.message ?? 'Patch failed');
  } finally {
    mod._patching = false;
  }
}
</script>

<style scoped>
.debug-panel {
  position: fixed;
  top: 0;
  right: 0;
  block-size: 100dvh;
  inline-size: var(--size-content-2);
  background: var(--surface-1);
  border-inline-start: var(--border-size-2) solid var(--surface-4);
  box-shadow: var(--shadow-6);
  display: flex;
  flex-direction: column;
  z-index: 100;
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

.debug-close {
  background: none;
  border: none;
  font-size: var(--font-size-2);
  cursor: pointer;
  color: var(--text-2);
  padding: var(--size-1);
}

.debug-close:hover {
  color: var(--text-1);
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
