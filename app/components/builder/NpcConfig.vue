<template>
  <div class="module-config">
    <Collapsible v-model="moduleOpen">
      <template #header>NPCs</template>
      <template #header-actions>
        <button type="button" class="btn-remove-module" @click.prevent="$emit('remove')">✕ Remove</button>
      </template>

      <div class="entry-list">
        <Collapsible v-for="(npc, i) in npcs" :key="i" v-model="npc._open">
          <template #header>
            <input v-model="npc.name" type="text" placeholder="NPC name" class="name-input" @click.stop />
          </template>
          <template #header-actions>
            <button type="button" class="btn-remove" @click.prevent="npcs.splice(i, 1)">✕</button>
          </template>
          <div class="entry-row">
            <select v-if="mapLocations.length > 0" v-model="npc.initialLocation">
              <option value="" disabled>Pick a location</option>
              <option v-for="loc in mapLocations" :key="loc" :value="loc">{{ loc }}</option>
            </select>
            <input v-else v-model="npc.initialLocation" type="text" placeholder="Initial location" />
            <select v-model="npc.disposition">
              <option value="neutral">Neutral</option>
              <option value="friendly">Friendly</option>
              <option value="hostile">Hostile</option>
            </select>
            <input v-model="npc.personality" type="text" placeholder="Personality (optional)" />
          </div>
        </Collapsible>
        <button type="button" class="btn-add" @click="npcs.push({ name: '', initialLocation: '', disposition: 'neutral' as const, personality: '', _open: true })">
          + Add NPC
        </button>
      </div>
    </Collapsible>
  </div>
</template>

<script setup lang="ts">
defineEmits<{ remove: [] }>();

const props = defineProps<{
  /** Configs from sibling modules, keyed by module type. */
  siblingConfigs?: Record<string, unknown>;
  /** Serialized config to hydrate from (e.g. from a saved draft). */
  initialConfig?: unknown;
}>();

const moduleOpen = ref(true);

const npcs = reactive<{
  name: string;
  initialLocation: string;
  disposition: 'hostile' | 'neutral' | 'friendly';
  personality: string;
  _open: boolean;
}[]>([
  { name: '', initialLocation: '', disposition: 'neutral', personality: '', _open: true },
]);

// Hydrate from initial config if provided
if (props.initialConfig) {
  const cfg = props.initialConfig as {
    npcs?: { name: string; initialLocation: string; disposition: string; personality?: string }[];
  };
  if (cfg.npcs?.length) {
    npcs.splice(0, npcs.length, ...cfg.npcs.map((n) => ({
      name: n.name,
      initialLocation: n.initialLocation ?? '',
      disposition: (n.disposition as 'hostile' | 'neutral' | 'friendly') ?? 'neutral',
      personality: n.personality ?? '',
      _open: true,
    })));
  }
}

/** Extract location names from the map::graph sibling module, if present. */
const mapLocations = computed(() => {
  const mapConfig = props.siblingConfigs?.['map::graph'] as {
    locations?: { name: string }[];
  } | undefined;
  return mapConfig?.locations?.map((l) => l.name).filter(Boolean) ?? [];
});

const validNpcs = computed(() => npcs.filter((n) => n.name.trim()));

const config = computed(() => {
  if (validNpcs.value.length === 0) return undefined;
  return {
    version: 1 as const,
    npcs: validNpcs.value.map((n) => ({
      name: n.name,
      initialLocation: n.initialLocation,
      disposition: n.disposition,
      personality: n.personality || undefined,
    })),
  };
});

const modelValue = defineModel<unknown>();
watch(config, (val) => { modelValue.value = val; }, { immediate: true });
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

.entry-list {
  display: flex;
  flex-direction: column;
  border-block-start: var(--border-size-1) solid var(--surface-3);
}

/* Entry-level collapsible overrides */
.entry-list :deep(.collapsible) {
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.entry-list :deep(.collapsible:last-child) {
  border-block-end: none;
}

.entry-list :deep(.collapsible) {
  --collapsible-header-padding: var(--size-2) var(--size-4);
  --collapsible-body-padding: 0 var(--size-4) var(--size-3);
}

.name-input {
  flex: 1;
  min-inline-size: 0;
}

.entry-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--size-2);
  align-items: center;
}

.entry-row input,
.entry-row select {
  flex: 1 1 var(--size-content-1);
  min-inline-size: 0;
}

.entry-row select {
  flex: 0 1 var(--size-xs);
}

.btn-remove {
  background: none;
  border: none;
  padding: var(--size-2);
  cursor: pointer;
  color: var(--text-2);
  font-size: var(--font-size-1);
  flex-shrink: 0;
}

.btn-remove:hover {
  color: var(--red-6);
}

.btn-add {
  background: none;
  border: var(--border-size-1) dashed var(--gray-6);
  border-radius: var(--radius-2);
  padding: var(--size-2);
  cursor: pointer;
  color: var(--gray-7);
  font-size: var(--font-size-1);
  margin-block-start: var(--size-2);
  transition: border-color var(--animation-duration, 0.2s) var(--ease-2),
    color var(--animation-duration, 0.2s) var(--ease-2);
}

.btn-add:hover {
  border-color: var(--indigo-6);
  color: var(--indigo-6);
}
</style>
