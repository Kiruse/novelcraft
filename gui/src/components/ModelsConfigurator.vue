<template>
  <section class="models-configurator">
    <h2 class="section-title">Models</h2>

    <p class="models-path">Your model configuration is saved to: <span class="models-path-value">{{ modelsPath }}</span></p>

    <div class="models-list">
      <ModelConfigBox
        v-for="(config, id) in models"
        :id="String(id)"
        :key="id"
        :config="config"
        :initially-unreachable="isHostUnreachable(config.base_url)"
        @save="onSave(String(id), $event)"
      />
    </div>
  </section>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { appDataDir } from '@tauri-apps/api/path';
import ModelConfigBox from '~/components/ModelConfigBox.vue';
import { useHostLiveness } from '~/composables/useHostLiveness';

interface ModelConfig {
  base_url: string;
  model_id: string;
  api_key?: string;
}

const { isHostUnreachable } = useHostLiveness();

const models = ref<Record<string, ModelConfig>>({});
const modelsPath = ref('');

onMounted(async () => {
  const dir = await appDataDir();
  const sep = dir.endsWith('/') || dir.endsWith('\\') ? '' : '/';
  modelsPath.value = `${dir}${sep}models.json`;
  models.value = await invoke<Record<string, ModelConfig>>('list_models');
});

async function onSave(id: string, updated: ModelConfig) {
  const all = { ...models.value, [id]: updated };
  await invoke('save_models', { models: all });
  models.value = all;
}
</script>

<style scoped>
.models-configurator {
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
}

.section-title {
  font-size: var(--font-size-3);
  font-weight: var(--font-weight-6);
  margin: 0;
}

.models-path {
  font-size: var(--font-size-0);
  color: var(--text-2);
  margin: 0;
}

.models-path-value {
  font-family: var(--font-mono);
}

.models-list {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}
</style>
