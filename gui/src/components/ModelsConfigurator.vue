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
import ModelConfigBox from '~/components/ModelConfigBox.vue';
import { useHostLiveness } from '~/composables/useHostLiveness';
import { commands, type ModelConfig, type ModelConfig_Deserialize } from '~/bindings';
import { unwrap } from '~/utils';

const { isHostUnreachable } = useHostLiveness();

const models = ref<Record<string, ModelConfig>>({});
const modelsPath = ref('');

async function onSave(id: string, updated: ModelConfig) {
  const all = { ...models.value, [id]: updated };
  await unwrap(commands.saveModels(all as Record<string, ModelConfig_Deserialize>));
  models.value = all;
}

onMounted(async () => {
  const [path, list] = await Promise.all([
    unwrap(commands.datapath('models.json')),
    unwrap(commands.listModels()),
  ]);
  modelsPath.value = path;
  models.value = list;
});
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
