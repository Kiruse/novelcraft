<template>
  <Collapsible
    v-model="expanded"
    :class="['model-config-box', { 'model-config-box--error': initiallyUnreachable && !pingError }]"
  >
    <template #header>
      <span class="model-header-id">{{ id }}</span>
      <span class="model-header-detail">{{ edit.model_id }} — {{ edit.base_url }}</span>
    </template>

    <div class="model-editor">
      <div class="field-group">
        <label class="field-label">Model ID</label>
        <input
          v-model="edit.model_id"
          class="field-value-input"
          placeholder="e.g. zai-org/glm-4.6v-flash"
        />
      </div>

      <div class="field-group">
        <label class="field-label">Base URL</label>
        <input
          ref="urlInputRef"
          v-model="edit.base_url"
          :class="['field-value-input', { 'field-value-input--error': pingError }]"
          placeholder="e.g. http://localhost:1234/v1"
          @input="onUrlChange"
        />
        <Tooltip :anchor="() => urlInputRef" :visible="!!pingError">
          {{ pingError }}
        </Tooltip>
      </div>

      <div class="field-group">
        <label class="field-label">API Key</label>
        <input
          v-model="edit.api_key"
          class="field-value-input"
          type="password"
          placeholder="Optional"
        />
      </div>

      <div class="editor-actions">
        <Spinner v-if="pinging" size="sm" label="Checking host" />
        <button class="btn btn--primary" :disabled="pinging" @click="save">Save</button>
        <button class="btn btn--ghost" @click="cancel">Cancel</button>
      </div>
    </div>
  </Collapsible>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import Collapsible from '~/components/Collapsible.vue';
import Spinner from '~/components/Spinner.vue';
import Tooltip from '~/components/Tooltip.vue';
import type { ModelConfig } from '~/bindings';

const props = defineProps<{
  id: string;
  config: ModelConfig;
  initiallyUnreachable: boolean;
}>();

const emit = defineEmits<{
  save: [config: ModelConfig];
}>();

const expanded = defineModel<boolean>({ default: false });

const urlInputRef = ref<HTMLInputElement | null>(null);

const edit = reactive({
  model_id: props.config.model_id,
  base_url: props.config.base_url,
  api_key: props.config.api_key ?? '',
});

const pinging = ref(false);
const pingError = ref<string | null>(null);

let pingTimer: ReturnType<typeof setTimeout> | null = null;

function onUrlChange() {
  if (pingTimer) clearTimeout(pingTimer);
  pingError.value = null;
  pinging.value = true;

  pingTimer = setTimeout(async () => {
    const url = edit.base_url.trim();
    if (!url) {
      pinging.value = false;
      return;
    }

    try {
      const error = await invoke<string | null>('ping_host', {
        request: { url, api_key: edit.api_key.trim() || null },
      });
      pingError.value = error;
    } catch {
      pingError.value = null;
    }
    pinging.value = false;
  }, 1000);
}

function cancel() {
  if (pingTimer) clearTimeout(pingTimer);
  pinging.value = false;
  pingError.value = null;
  edit.model_id = props.config.model_id;
  edit.base_url = props.config.base_url;
  edit.api_key = props.config.api_key ?? '';
  expanded.value = false;
}

function save() {
  if (pingTimer) clearTimeout(pingTimer);
  pinging.value = false;

  const updated: ModelConfig = {
    base_url: edit.base_url.trim(),
    model_id: edit.model_id.trim(),
  };
  const apiKey = edit.api_key.trim();
  if (apiKey) {
    updated.api_key = apiKey;
  }

  emit('save', updated);
  expanded.value = false;
}
</script>

<style scoped>
.model-config-box {
  --collapsible-header-padding: var(--size-3) var(--size-4);
  background: var(--surface-1);
  border: var(--border-size-1) solid var(--surface-3);
  border-radius: var(--radius-2);
}

.model-config-box--error {
  border-color: var(--red-6);
}

.model-header-id {
  margin-right: var(--size-1);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  color: var(--text-1);
}

.model-header-detail {
  font-size: var(--font-size-0);
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.model-editor {
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
}

.field-group {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.field-label {
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-6);
  color: var(--text-2);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.field-value-input {
  padding: var(--size-2) var(--size-3);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  background: var(--surface-1);
  color: var(--text-1);
  font-size: var(--font-size-2);
  font-family: inherit;
  transition: border-color var(--animation-duration, 0.15s) var(--ease-2);
}

.field-value-input:focus {
  outline: none;
  border-color: var(--indigo-6);
}

.field-value-input::placeholder {
  color: var(--text-2);
  opacity: 0.5;
}

.field-value-input--error {
  border-color: var(--red-6);
}

.field-value-input--error:focus {
  border-color: var(--red-6);
}

.editor-actions {
  display: flex;
  gap: var(--size-2);
  align-items: center;
  justify-content: flex-end;
}

.btn {
  padding: var(--size-2) var(--size-5);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  border: none;
  font-family: inherit;
}

.btn--primary {
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.btn--primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn--ghost {
  background: transparent;
  color: var(--text-2);
}

.btn--ghost:hover {
  color: var(--text-1);
  background: var(--surface-3);
}
</style>
