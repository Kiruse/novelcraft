<template>
  <div class="settings-page">
    <h1 class="settings-title">Settings</h1>

    <section class="settings-section">
      <h2 class="section-title">Models</h2>

      <div class="models-list">
        <div
          v-for="(config, id) in models"
          :key="id"
          class="model-card"
          role="button"
          tabindex="0"
          @click="startEdit(String(id), config)"
          @keydown.enter="startEdit(String(id), config)"
        >
          <div class="model-card-info">
            <span class="model-card-id">{{ id }}</span>
            <span class="model-card-url">{{ config.base_url }}</span>
          </div>
          <button class="model-card-edit" title="Edit model" @click.stop="startEdit(String(id), config)">
            <EditOutlined />
          </button>
          <button class="model-card-remove" title="Delete model" @click.stop="pendingDeleteId = String(id)">
            &times;
          </button>
        </div>
      </div>

      <div v-if="editing" class="model-editor">
        <div class="field-group">
          <label class="field-label">Model ID</label>
          <input v-model="editId" class="field-value-input" placeholder="e.g. qwen/qwen3.5-9b" />
        </div>

        <div class="field-group">
          <label class="field-label">Base URL</label>
          <input v-model="editBaseUrl" class="field-value-input" placeholder="e.g. http://localhost:1234/v1" />
        </div>

        <div class="field-group">
          <label class="field-label">API Key</label>
          <input v-model="editApiKey" class="field-value-input" type="password" placeholder="Optional" />
        </div>

        <div class="editor-actions">
          <button class="btn btn--primary" @click="saveModel">Save</button>
          <button class="btn btn--ghost" @click="cancelEdit">Cancel</button>
        </div>
      </div>

      <button v-else class="add-model-btn" @click="startCreate">+ Add model</button>
    </section>

    <Teleport to="body">
      <div v-if="pendingDeleteId" class="confirm-overlay" @click.self="pendingDeleteId = null">
        <div class="confirm-box">
          <p class="confirm-text">Delete this model?</p>
          <div class="confirm-actions">
            <button class="btn btn--danger" @click="confirmDelete">Delete</button>
            <button class="btn btn--ghost" @click="pendingDeleteId = null">Cancel</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { EditOutlined } from '@ant-design/icons-vue';

interface ModelConfig {
  base_url: string;
  api_key?: string;
}

const models = ref<Record<string, ModelConfig>>({});
const editing = ref(false);
const editOriginalId = ref('');
const editId = ref('');
const editBaseUrl = ref('');
const editApiKey = ref('');
const pendingDeleteId = ref<string | null>(null);

onMounted(async () => {
  models.value = await invoke<Record<string, ModelConfig>>('list_models');
});

function startEdit(id: string, config: ModelConfig) {
  editOriginalId.value = id;
  editId.value = id;
  editBaseUrl.value = config.base_url;
  editApiKey.value = config.api_key ?? '';
  editing.value = true;
}

function startCreate() {
  editOriginalId.value = '';
  editId.value = '';
  editBaseUrl.value = '';
  editApiKey.value = '';
  editing.value = true;
}

function cancelEdit() {
  editing.value = false;
}

async function saveModel() {
  const id = editId.value.trim();
  const baseUrl = editBaseUrl.value.trim();
  if (!id || !baseUrl) return;

  const updated = { ...models.value };

  if (editOriginalId.value && editOriginalId.value !== id) {
    delete updated[editOriginalId.value];
  }

  const config: ModelConfig = { base_url: baseUrl };
  if (editApiKey.value.trim()) {
    config.api_key = editApiKey.value.trim();
  }

  updated[id] = config;

  await invoke('save_models', { models: updated });
  models.value = updated;
  editing.value = false;
}

async function confirmDelete() {
  if (!pendingDeleteId.value) return;

  const updated = { ...models.value };
  delete updated[pendingDeleteId.value];

  await invoke('save_models', { models: updated });
  models.value = updated;
  pendingDeleteId.value = null;
}
</script>

<style scoped>
.settings-page {
  max-inline-size: var(--size-content-3);
  margin-inline: auto;
  padding: var(--size-8);
}

.settings-title {
  font-size: var(--font-size-6);
  font-weight: var(--font-weight-7);
  margin-block-end: var(--size-8);
}

.section-title {
  font-size: var(--font-size-3);
  font-weight: var(--font-weight-6);
  margin: 0 0 var(--size-4);
}

.models-list {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.model-card {
  display: flex;
  align-items: center;
  gap: var(--size-3);
  padding: var(--size-3) var(--size-4);
  border-radius: var(--radius-2);
  background: var(--surface-1);
  border: var(--border-size-1) solid var(--surface-3);
  cursor: pointer;
  transition: background var(--animation-duration, 0.15s) var(--ease-2);
}

.model-card:hover {
  background: var(--surface-3);
}

.model-card-info {
  flex: 1;
  min-inline-size: 0;
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
}

.model-card-id {
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  color: var(--text-1);
}

.model-card-url {
  font-size: var(--font-size-0);
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.model-card-edit {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-2);
  color: var(--text-2);
  padding: var(--size-1);
  line-height: 1;
  flex-shrink: 0;
}

.model-card-edit:hover {
  color: var(--indigo-6);
}

.model-card-remove {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-2);
  color: var(--text-2);
  padding: var(--size-1);
  line-height: 1;
  flex-shrink: 0;
}

.model-card-remove:hover {
  color: var(--red-6);
}

.model-editor {
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
  padding-block-start: var(--size-3);
  border-block-start: var(--border-size-1) solid var(--surface-3);
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

.add-model-btn {
  display: block;
  width: 100%;
  padding: var(--size-3);
  margin-top: var(--size-2);
  border-radius: var(--radius-2);
  background: var(--surface-3);
  color: var(--text-1);
  border: var(--border-size-1) dashed var(--surface-4);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-5);
  cursor: pointer;
  font-family: inherit;
  text-align: start;
  transition: background var(--animation-duration, 0.15s) var(--ease-2);
}

.add-model-btn:hover {
  background: var(--surface-4);
}

.editor-actions {
  display: flex;
  gap: var(--size-2);
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

.btn--ghost {
  background: transparent;
  color: var(--text-2);
}

.btn--ghost:hover {
  color: var(--text-1);
  background: var(--surface-3);
}

.btn--danger {
  background: var(--red-6);
  color: var(--gray-0);
}

.btn--danger:hover {
  opacity: 0.9;
}

.confirm-overlay {
  position: fixed;
  inset: 0;
  background: oklch(from var(--gray-9) l c h / 0.5);
  z-index: var(--layer-5, 200);
  display: flex;
  align-items: center;
  justify-content: center;
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
</style>
