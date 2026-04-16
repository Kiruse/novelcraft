<template>
  <div class="module-config">
    <Collapsible v-model="moduleOpen">
      <template #header>Map (graph)</template>
      <template #header-actions>
        <button type="button" class="btn-remove-module" @click.prevent="$emit('remove')">✕ Remove</button>
      </template>

      <label>
        Starting location
        <select v-model="startingLocation">
          <option value="" disabled>Pick a location (add one first)</option>
          <option v-for="loc in validLocations" :key="loc.name" :value="loc.name">
            {{ loc.name }}
          </option>
        </select>
      </label>

      <div class="entry-list">
        <Collapsible v-for="(loc, i) in locations" :key="i" v-model="loc._open">
          <template #header>
            <input v-model="loc.name" type="text" placeholder="Location name" class="name-input" @click.stop />
          </template>
          <template #header-actions>
            <button type="button" class="btn-remove" @click.prevent="removeLocation(i)">✕</button>
          </template>
          <textarea
            v-model="loc.description"
            placeholder="Describe this location…"
            rows="3"
            class="description-input"
          />
        </Collapsible>
        <button type="button" class="btn-add" @click="addLocation">
          + Add location
        </button>
      </div>
    </Collapsible>
  </div>
</template>

<script setup lang="ts">
defineEmits<{ remove: [] }>();

const props = defineProps<{
  /** Serialized config to hydrate from (e.g. from a saved draft). */
  initialConfig?: unknown;
}>();

const locations = reactive<{ name: string; description: string; _open: boolean }[]>([
  { name: '', description: '', _open: true },
]);
const startingLocation = ref('');
const moduleOpen = ref(true);

// Hydrate from initial config if provided
if (props.initialConfig) {
  const cfg = props.initialConfig as {
    locations?: { name: string; description: string }[];
    _startingLocation?: string;
  };
  if (cfg.locations?.length) {
    locations.splice(0, locations.length, ...cfg.locations.map((l) => ({
      name: l.name,
      description: l.description ?? '',
      _open: true,
    })));
  }
  if (cfg._startingLocation) {
    startingLocation.value = cfg._startingLocation;
  }
}

const validLocations = computed(() => locations.filter((l) => l.name.trim()));

const config = computed(() => {
  if (validLocations.value.length === 0) return undefined;

  return {
    version: 1 as const,
    locations: validLocations.value.map((l) => ({
      name: l.name,
      description: l.description,
      connections: validLocations.value
        .filter((o) => o.name !== l.name)
        .reduce((acc, o) => ({ ...acc, [o.name]: 100 }), {} as Record<string, number>),
    })),
    _startingLocation: startingLocation.value || validLocations.value[0]?.name || '',
  };
});

const modelValue = defineModel<unknown>();
watch(config, (val) => { modelValue.value = val; }, { immediate: true });

function addLocation() {
  locations.push({ name: '', description: '', _open: true });
}

function removeLocation(index: number) {
  const removed = locations[index];
  locations.splice(index, 1);
  if (removed && startingLocation.value === removed.name) {
    startingLocation.value = '';
  }
}
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

label {
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
  margin-block: var(--size-3);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  color: var(--text-2);
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

.description-input {
  resize: vertical;
  min-block-size: 3lh;
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
