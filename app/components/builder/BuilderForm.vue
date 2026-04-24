<template>
  <form class="builder-form" @submit.prevent="publish">
    <slot name="header" />

    <!-- Basic info -->
    <div class="section-divider">Basic Info</div>
    <div class="form-section">
      <label>
        Story ID
        <input v-model="form.storyId" type="text" placeholder="my-cool-story" :disabled="storyIdDisabled" />
      </label>
      <label>
        Title
        <input v-model="form.title" type="text" placeholder="The Lost Dungeon" />
      </label>
      <label>
        Genre
        <input v-model="form.genre" type="text" placeholder="Fantasy" />
      </label>
      <label>
        Cover Art URL
        <input v-model="form.coverArt" type="url" placeholder="https://..." />
      </label>
      <label>
        Description
        <textarea v-model="form.description" rows="3" placeholder="A brief description of your story..." />
        <span v-if="form.description" class="char-count" :class="{ 'char-count--warn': form.description.trim().length < 100 }">
          {{ form.description.trim().length }}/100
        </span>
      </label>

      <button type="button" class="btn-inspire" @click.prevent="inspireRef?.open()">
        ✨ Get inspired
      </button>
    </div>

    <!-- Modules -->
    <div class="section-divider">Modules</div>

    <div class="form-section">
      <div class="module-picker">
        <button
          v-if="availableModules.length > 0"
          type="button"
          class="btn-add-module"
          @click="openModuleDialog"
        >
          + Add module
        </button>
      </div>

      <p v-if="activeModuleTypes.size === 0" class="hint">
        No modules added yet.
      </p>

      <component
        v-for="[type, component] in activeConfigComponents"
        :key="type"
        :is="component"
        :model-value="modulesConfig[type]"
        :initial-config="initialModulesConfig?.[type]"
        :sibling-configs="modulesConfig"
        @update:model-value="(v: unknown) => setModuleConfig(type, v)"
        @remove="removeModule(type)"
      />
    </div>

    <!-- Inspire dialog -->
    <BuilderInspireDialog ref="inspireRef" @use="applySuggestion" />

    <!-- Module picker dialog -->
    <dialog ref="moduleDialogEl" class="module-dialog">
      <div class="dialog-panel">
        <h2 class="dialog-title">Add a module</h2>
        <ul class="dialog-list">
          <li v-for="mod in availableModules" :key="mod.type" class="dialog-item">
            <span class="dialog-item-name">{{ mod.type }}</span>
            <button type="button" class="dialog-item-add" @click="addModule(mod.type)">Add</button>
          </li>
          <li v-if="availableModules.length === 0" class="dialog-empty">
            All modules have been added.
          </li>
        </ul>
        <div class="dialog-actions">
          <button type="button" class="dialog-close" @click="closeModuleDialog">Close</button>
        </div>
      </div>
    </dialog>

    <!-- JSON preview -->
    <div class="section-divider">Module Config Preview</div>
    <pre class="json-preview">{{ modulesJson }}</pre>

    <div class="form-actions">
      <button type="button" class="btn-draft" :class="{ 'is-test': !isDirty && hasDraft }" :disabled="saving || !canSaveDraft" @click.prevent="onSaveOrTest">
        {{ saving ? 'Saving...' : (!isDirty && hasDraft) ? '▶ Test' : 'Save Draft' }}
      </button>
      <button type="submit" class="btn-publish" :disabled="submitting || !canPublish">
        {{ submitting ? 'Publishing...' : 'Publish' }}
      </button>
    </div>

    <div v-if="publishErrors.length > 0" class="validation-hint">
      <p>To publish, you still need:</p>
      <ul>
        <li v-for="err in publishErrors" :key="err">{{ err }}</li>
      </ul>
    </div>

    <div v-if="result" class="result">
      <slot name="result" :result="result">
        <p>✅ <NuxtLink :to="`/stories/${result.authorName}/${result.storyId}`">{{ result.title }}</NuxtLink></p>
      </slot>
    </div>
  </form>
</template>

<script setup lang="ts">
const props = defineProps<{
  storyIdDisabled?: boolean;
  initialModulesConfig?: Record<string, unknown>;
}>();

const {
  form,
  activeModuleTypes,
  modulesConfig,
  moduleDialogEl,
  submitting,
  saving,
  result,
  isDirty,
  hasDraft,
  publishErrors,
  canSaveDraft,
  canPublish,
  availableModules,
  activeConfigComponents,
  modulesJson,
  addModule,
  removeModule,
  setModuleConfig,
  openModuleDialog,
  closeModuleDialog,
  populateFrom,
  saveDraft,
  publish,
} = useStoryBuilder();

const inspireRef = ref<{ open: () => void } | null>(null);

function applySuggestion(s: Record<string, string>) {
  if (!props.storyIdDisabled && s.storyId) {
    form.storyId = s.storyId;
  }
  if (s.title) form.title = s.title;
  if (s.genre) form.genre = s.genre;
  if (s.description) form.description = s.description;
}

async function onSaveOrTest() {
  if (isDirty.value) {
    // Save draft first
    const saved = await saveDraft();
    if (!saved) return;
  }
  // Launch test session
  await navigateToTest();
}

async function navigateToTest() {
  if (!result.value?.id) return;
  await navigateTo(`/stories/${result.value.authorName}/${result.value.storyId}?test=1`);
}

defineExpose({ populateFrom, form, saveDraft, publish, isDirty, hasDraft, result });
</script>

<style scoped>
.builder-form {
  /* no-op — allows parent to control layout */
}

.section-divider {
  display: flex;
  align-items: center;
  gap: var(--size-3);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  color: var(--text-2);
  margin-block: var(--size-6) var(--size-4);
}

.section-divider::before,
.section-divider::after {
  content: '';
  flex: 1;
  block-size: var(--border-size-1);
  background: var(--surface-4);
}

.form-section {
  margin-block-end: var(--size-4);
}

label {
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
  margin-block-end: var(--size-3);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  color: var(--text-2);
}

/* Module picker */
.module-picker {
  margin-block-end: var(--size-4);
}

.btn-add-module {
  padding: var(--size-2) var(--size-5);
  background: var(--brand-gradient);
  color: var(--gray-0);
  border: none;
  border-radius: var(--radius-2);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  white-space: nowrap;
}

.hint {
  color: var(--text-2);
  font-size: var(--font-size-1);
  font-style: italic;
  margin: 0;
}

/* Module dialog */
.module-dialog {
  border: none;
  padding: 0;
  background: none;
  max-inline-size: var(--size-lg);
  inline-size: 100%;
}

.module-dialog::backdrop {
  background: oklch(0 0 0 / 0.4);
}

.dialog-panel {
  background: var(--surface-1);
  border-radius: var(--radius-3);
  box-shadow: var(--shadow-6);
  padding: var(--size-6);
}

.dialog-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-4);
}

.dialog-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.dialog-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-3) var(--size-4);
  background: var(--surface-2);
  border-radius: var(--radius-2);
}

.dialog-item-name {
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-5);
}

.dialog-item-add {
  padding: var(--size-1) var(--size-4);
  background: var(--brand-gradient);
  color: var(--gray-0);
  border: none;
  border-radius: var(--radius-1);
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-6);
  cursor: pointer;
}

.dialog-item-add:hover {
  opacity: 0.9;
}

.dialog-empty {
  padding: var(--size-4);
  text-align: center;
  color: var(--text-2);
  font-style: italic;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  margin-block-start: var(--size-5);
}

.dialog-close {
  padding: var(--size-2) var(--size-6);
  background: var(--surface-3);
  color: var(--text-1);
  border: none;
  border-radius: var(--radius-2);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  cursor: pointer;
}

.dialog-close:hover {
  background: var(--surface-4);
}

/* Preview */
.json-preview {
  background: var(--surface-2);
  border-radius: var(--radius-2);
  padding: var(--size-4);
  font-size: var(--font-size-0);
  line-height: var(--font-lineheight-4);
  overflow-x: auto;
  white-space: pre-wrap;
}

/* Actions */
.form-actions {
  display: flex;
  gap: var(--size-3);
  margin-block-start: var(--size-4);
}

.btn-draft {
  background: var(--surface-3);
  color: var(--text-1);
  border: var(--border-size-1) solid var(--surface-4);
  padding: var(--size-3) var(--size-6);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  border-radius: var(--radius-2);
  cursor: pointer;
}

.btn-draft:hover:not(:disabled) {
  background: var(--surface-4);
}

.btn-draft:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* When button shows "▶ Test" — clean draft ready to playtest */
.btn-draft.is-test {
  background: var(--green-9);
  color: var(--green-2);
  border-color: var(--green-6);
}

.btn-draft.is-test:hover:not(:disabled) {
  background: var(--green-8);
}

.btn-publish {
  background: var(--brand-gradient);
  color: var(--gray-0);
  border: none;
  padding: var(--size-3) var(--size-8);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  border-radius: var(--radius-2);
  cursor: pointer;
  transition: transform var(--animation-duration, 0.2s) var(--ease-2),
    box-shadow var(--animation-duration, 0.2s) var(--ease-2);
}

.char-count {
  font-size: var(--font-size-0);
  color: var(--text-2);
  text-align: end;
}

.char-count--warn {
  color: var(--orange-6);
}

.btn-inspire {
  background: none;
  border: var(--border-size-1) dashed var(--indigo-6);
  border-radius: var(--radius-2);
  padding: var(--size-2) var(--size-4);
  color: var(--indigo-6);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  cursor: pointer;
  transition: background var(--animation-duration, 0.2s) var(--ease-2);
}

.btn-inspire:hover {
  background: var(--indigo-0);
}

.btn-publish:hover:not(:disabled) {
  transform: translateY(calc(var(--size-1) * -1));
  box-shadow: var(--shadow-3);
}

.btn-publish:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.result {
  margin-block-start: var(--size-4);
  padding: var(--size-4);
  background: var(--surface-2);
  color: var(--text-2);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
}

.result p {
  margin: 0;
}

.result :deep(a) {
  color: var(--indigo-6);
  font-weight: var(--font-weight-6);
}

/* Validation */
.validation-hint {
  margin-block-start: var(--size-4);
  padding: var(--size-4);
  background: var(--surface-2);
  border-radius: var(--radius-2);
  border: var(--border-size-1) solid var(--surface-4);
  color: var(--text-2);
  font-size: var(--font-size-1);
}

.validation-hint p {
  margin: 0 0 var(--size-2);
  font-weight: var(--font-weight-5);
}

.validation-hint ul {
  margin: 0;
  padding-inline-start: var(--size-4);
}

.validation-hint li {
  margin-block-end: var(--size-1);
}
</style>
