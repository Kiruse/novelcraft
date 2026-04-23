<template>
  <div class="module-config">
    <Collapsible v-model="moduleOpen">
      <template #header>System Prompt</template>
      <template #header-actions>
        <button type="button" class="btn-remove-module" @click.prevent="$emit('remove')">✕ Remove</button>
      </template>

      <label class="field-label">
        <span class="label-row">
          Prompt
          <InfoCircleOutlined class="info-icon" title="Hidden general story-relevant information always shown to your agent, but not the players. Can be used to e.g. define the story teller's style." />
        </span>
        <textarea
          v-model="prompt"
          rows="6"
          placeholder="You are a dramatic fantasy narrator who speaks in second person. You favor vivid sensory descriptions and never break character..."
        />
      </label>
    </Collapsible>
  </div>
</template>

<script setup lang="ts">
import { InfoCircleOutlined } from '@ant-design/icons-vue';

defineEmits<{ remove: [] }>();

const props = defineProps<{
  initialConfig?: unknown;
}>();

const moduleOpen = ref(true);
const prompt = ref('');

if (props.initialConfig && typeof props.initialConfig === 'object' && 'prompt' in (props.initialConfig as object)) {
  prompt.value = (props.initialConfig as { prompt: string }).prompt ?? '';
}

const config = computed(() => {
  if (!prompt.value.trim()) return undefined;
  return { version: 1 as const, prompt: prompt.value };
});

const modelValue = defineModel<unknown>();
watch(config, (val) => { modelValue.value = val; }, { immediate: true, deep: true });
</script>

<style scoped>
.module-config {
  margin-block-start: var(--size-4);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  overflow: hidden;
}

.btn-remove-module {
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-4);
  padding: var(--size-1) var(--size-2);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-1);
  background: var(--surface-2);
  color: var(--text-2);
  cursor: pointer;
  flex-shrink: 0;
}

.btn-remove-module:hover {
  background: var(--red-9);
  color: var(--red-2);
  border-color: var(--red-6);
}

.field-label {
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
  margin-block-end: var(--size-2);
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-5);
  color: var(--text-2);
}

.label-row {
  display: flex;
  align-items: center;
  gap: var(--size-2);
}

.info-icon {
  font-size: var(--font-size-0);
  color: var(--text-2);
  opacity: 0.6;
  cursor: help;
}

.info-icon:hover {
  opacity: 1;
}
</style>
