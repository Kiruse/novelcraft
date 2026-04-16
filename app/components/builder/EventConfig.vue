<template>
  <div class="module-config">
    <Collapsible v-model="moduleOpen">
      <template #header>Events</template>
      <template #header-actions>
        <button type="button" class="btn-remove-module" @click.prevent="$emit('remove')">✕ Remove</button>
      </template>

      <!-- Variables -->
      <div class="subsection">
        <div class="subsection-header">
          <span class="subsection-title">Variables</span>
          <button type="button" class="btn-sm" @click="addVariable">+ Variable</button>
        </div>
        <div v-for="(v, i) in variables" :key="i" class="var-row">
          <input v-model="v.name" type="text" placeholder="variable_name" class="var-name" />
          <label class="toggle-label">
            <input v-model="v.defaultValue" type="checkbox" />
            Default: {{ v.defaultValue ? 'true' : 'false' }}
          </label>
          <button type="button" class="btn-remove-sm" @click="variables.splice(i, 1)">✕</button>
        </div>
        <p v-if="variables.length === 0" class="hint">No variables defined yet.</p>
      </div>

      <!-- Events -->
      <div class="subsection">
        <div class="subsection-header">
          <span class="subsection-title">Events</span>
          <button type="button" class="btn-sm" @click="addEvent">+ Event</button>
        </div>

        <div class="entry-list">
          <Collapsible v-for="(evt, ei) in events" :key="ei" v-model="evt._open">
            <template #header>
              <input v-model="evt.name" type="text" placeholder="Event name" class="name-input" @click.stop />
            </template>
            <template #header-actions>
              <button type="button" class="btn-remove" @click.prevent="events.splice(ei, 1)">✕</button>
            </template>

            <label class="field-label">
              ID
              <input v-model="evt.id" type="text" placeholder="event-id" />
            </label>

            <label class="field-label">
              Description
              <textarea v-model="evt.description" placeholder="What happens when this event fires…" rows="2" />
            </label>

            <!-- Conditions -->
            <Collapsible v-model="evt._conditionsOpen">
              <template #header>Conditions</template>
              <template #header-actions>
                <button type="button" class="btn-sm" @click.stop="addRule(evt.conditions)">+</button>
              </template>
              <div v-for="(cond, ci) in evt.conditions" :key="ci" class="rule-card">
                <div class="rule-header">
                  <span class="rule-index">Condition #{{ ci + 1 }}</span>
                  <button type="button" class="btn-remove-sm" @click="evt.conditions.splice(ci, 1)">✕</button>
                </div>
                <input v-model="cond.description" type="text" placeholder="e.g. On day 5, between 08:00 and 18:00" />
                <textarea
                  v-model="cond.script"
                  placeholder="day == 5 and time >= 08:00 and time <= 18:00"
                  rows="2"
                  class="script-input"
                />
              </div>
              <button v-if="evt.conditions.length === 0" type="button" class="btn-sm" @click="addRule(evt.conditions)">+ Condition</button>
            </Collapsible>

            <!-- Triggers -->
            <Collapsible v-model="evt._triggersOpen">
              <template #header>Triggers</template>
              <template #header-actions>
                <button type="button" class="btn-sm" @click.stop="addRule(evt.triggers)">+</button>
              </template>
              <div v-for="(trig, ti) in evt.triggers" :key="ti" class="rule-card">
                <div class="rule-header">
                  <span class="rule-index">Trigger #{{ ti + 1 }}</span>
                  <button type="button" class="btn-remove-sm" @click="evt.triggers.splice(ti, 1)">✕</button>
                </div>
                <input v-model="trig.description" type="text" placeholder="e.g. when the player enters the throne room" />
                <textarea
                  v-model="trig.script"
                  placeholder="set alarm_triggered to true"
                  rows="2"
                  class="script-input"
                />
              </div>
              <button v-if="evt.triggers.length === 0" type="button" class="btn-sm" @click="addRule(evt.triggers)">+ Trigger</button>
            </Collapsible>
          </Collapsible>
        </div>

        <p v-if="events.length === 0" class="hint">No events defined yet.</p>
        <button type="button" class="btn-add" @click="addEvent">+ Add event</button>
      </div>
    </Collapsible>
  </div>
</template>

<script setup lang="ts">
defineEmits<{ remove: [] }>();

const props = defineProps<{
  initialConfig?: unknown;
}>();

const moduleOpen = ref(true);

// --- Types ---

interface Rule {
  description: string;
  script: string;
}

interface Event {
  id: string;
  name: string;
  description: string;
  conditions: Rule[];
  triggers: Rule[];
  _open: boolean;
  _conditionsOpen: boolean;
  _triggersOpen: boolean;
}

interface Variable {
  name: string;
  defaultValue: boolean;
}

// --- State ---

const variables = reactive<Variable[]>([]);
const events = reactive<Event[]>([]);

// --- Hydrate ---

if (props.initialConfig) {
  const cfg = props.initialConfig as {
    events?: Record<string, unknown>[];
    variables?: { name: string; defaultValue?: boolean }[];
  };
  if (cfg.variables?.length) {
    variables.push(...cfg.variables.map((v) => ({ name: v.name, defaultValue: v.defaultValue ?? false })));
  }
  if (cfg.events?.length) {
    events.push(...cfg.events.map((e) => ({
      id: (e.id as string) ?? '',
      name: (e.name as string) ?? '',
      description: (e.description as string) ?? '',
      conditions: (e.conditions as Rule[]) ?? [],
      triggers: (e.triggers as Rule[]) ?? [],
      _open: true,
      _conditionsOpen: false,
      _triggersOpen: false,
    })));
  }
}

// --- Actions ---

function addVariable() {
  variables.push({ name: '', defaultValue: false });
}

function addEvent() {
  events.push({
    id: '',
    name: '',
    description: '',
    conditions: [],
    triggers: [],
    _open: true,
    _conditionsOpen: false,
    _triggersOpen: false,
  });
}

function addRule(list: Rule[]) {
  list.push({ description: '', script: '' });
}

// --- Serialize ---

const validEvents = computed(() =>
  events.filter((e) => e.name.trim() && e.id.trim()),
);

const config = computed(() => {
  const hasEvents = validEvents.value.length > 0;
  const hasVariables = variables.some((v) => v.name.trim());
  if (!hasEvents && !hasVariables) return undefined;

  return {
    version: 1 as const,
    events: validEvents.value.map((e) => ({
      id: e.id,
      name: e.name,
      description: e.description,
      conditions: e.conditions,
      triggers: e.triggers,
    })),
    variables: variables
      .filter((v) => v.name.trim())
      .map((v) => ({ name: v.name, defaultValue: v.defaultValue })),
  };
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

/* Subsections */
.subsection {
  margin-block: var(--size-3);
}

.subsection-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-block-end: var(--size-2);
}

.subsection-title {
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-6);
  color: var(--text-2);
}

/* Buttons */
.btn-sm {
  background: none;
  border: var(--border-size-1) dashed var(--gray-6);
  border-radius: var(--radius-1);
  padding: var(--size-1) var(--size-3);
  cursor: pointer;
  color: var(--gray-7);
  font-size: var(--font-size-0);
  transition: border-color var(--animation-duration, 0.2s) var(--ease-2),
    color var(--animation-duration, 0.2s) var(--ease-2);
}

.btn-sm:hover {
  border-color: var(--indigo-6);
  color: var(--indigo-6);
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

.btn-remove-sm {
  background: none;
  border: none;
  padding: var(--size-1);
  cursor: pointer;
  color: var(--text-2);
  font-size: var(--font-size-0);
}

.btn-remove-sm:hover {
  color: var(--red-6);
}

/* Field labels */
.field-label {
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
  margin-block-end: var(--size-2);
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-5);
  color: var(--text-2);
}

.hint {
  color: var(--text-2);
  font-size: var(--font-size-0);
  font-style: italic;
  margin: 0;
  padding: var(--size-1) 0;
}

/* Variable rows */
.var-row {
  display: flex;
  align-items: center;
  gap: var(--size-2);
  margin-block-end: var(--size-1);
}

.var-name {
  flex: 1;
  min-inline-size: 0;
}

.toggle-label {
  display: inline-flex;
  align-items: center;
  gap: var(--size-1);
  font-size: var(--font-size-0);
  color: var(--text-2);
  cursor: pointer;
  white-space: nowrap;
}

/* Entry list (events) */
.entry-list {
  display: flex;
  flex-direction: column;
}

.entry-list :deep(.collapsible) {
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.entry-list :deep(.collapsible:last-child) {
  border-block-end: none;
}

.entry-list :deep(> .collapsible) {
  --collapsible-header-padding: var(--size-2) var(--size-4);
  --collapsible-body-padding: 0 var(--size-4) var(--size-3);
}

/* Nested collapsibles (conditions, triggers) — no extra padding */
.entry-list :deep(.collapsible-body-inner > .collapsible) {
  --collapsible-header-padding: 0;
  --collapsible-body-padding: 0;
}

.name-input {
  flex: 1;
  min-inline-size: 0;
}

/* Rule cards (shared for conditions & triggers) */
.rule-card {
  background: var(--surface-2);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  padding: var(--size-2) var(--size-3);
  margin-block-end: var(--size-2);
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.rule-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.rule-index {
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-6);
  color: var(--text-2);
}

.script-input {
  font-family: var(--font-mono, monospace);
  font-size: var(--font-size-0);
  resize: vertical;
  min-block-size: 2lh;
}
</style>
