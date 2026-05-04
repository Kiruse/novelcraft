<template>
  <Teleport to="body">
    <div v-if="open" class="dialog-overlay" @click.self="$emit('close')">
      <div class="dialog-panel">
        <div class="dialog-header">
          <h2 class="dialog-title">Profiles</h2>
          <button class="dialog-close" @click="$emit('close')">&times;</button>
        </div>

        <div class="dialog-body">
          <div class="profiles-list">
            <div
              v-for="p in profiles"
              :key="p.id"
              class="profile-card"
              :class="{ 'profile-card--active': p.active }"
              role="button"
              tabindex="0"
              @click="selectProfile(p.id)"
              @keydown.enter="selectProfile(p.id)"
            >
              <span class="profile-card-dot" />
              <div class="profile-card-info">
                <span class="profile-card-name">{{ p.name }}</span>
                <span class="profile-card-preview">{{ profilePreview(p) }}</span>
              </div>
              <button class="profile-card-edit" title="Edit profile" @click.stop="startEdit(p)">
                <EditOutlined />
              </button>
              <button class="profile-card-remove" title="Delete profile" @click.stop="pendingDeleteId = p.id">
                &times;
              </button>
            </div>
          </div>

          <div v-if="editingProfile" class="profile-editor">
            <div class="field-group">
              <label class="field-label">Profile name</label>
              <input
                v-model="editName"
                class="field-input"
                placeholder="Profile name"
                @keydown.enter="saveProfile"
              />
            </div>

            <div class="field-group">
              <label class="field-label">Fields</label>
              <div class="fields-list">
                <div
                  v-for="(pair, idx) in editFields"
                  :key="idx"
                  class="field-row"
                  :class="{ 'field-row--drag-over': dragOverIdx === idx }"
                  draggable="true"
                  @dragstart="onDragStart(idx)"
                  @dragover.prevent="onDragOver(idx)"
                  @dragleave="dragOverIdx = -1"
                  @drop.prevent="onDrop(idx)"
                  @dragend="dragOverIdx = -1"
                >
                  <span class="field-grip" aria-hidden="true"><HolderOutlined /></span>
                  <input
                    v-model="pair.key"
                    class="field-key-input"
                    placeholder="key"
                    @keydown.tab="onKeyTab($event, idx)"
                    @keydown.enter.prevent="onKeyEnter(idx)"
                  />
                  <span class="field-sep">:</span>
                  <textarea
                    v-if="pair.textarea"
                    ref="valueRefs"
                    v-model="pair.value"
                    class="field-value-input field-value-input--expanded"
                    placeholder="value"
                    rows="3"
                    @keydown="onType($event, idx)"
                    @keydown.tab="onValueTab($event, idx)"
                    @keydown.enter.ctrl.prevent="onValueEnter(idx)"
                  />
                  <input
                    v-else
                    ref="valueRefs"
                    v-model="pair.value"
                    class="field-value-input"
                    placeholder="value"
                    @keydown="onType($event, idx)"
                    @keydown.tab="onValueTab($event, idx)"
                    @keydown.enter.prevent="onValueEnter(idx)"
                  />
                  <button class="field-remove" title="Remove field" @click="removeField(idx)">&times;</button>
                </div>
              </div>
              <button class="add-field-btn" @click="addField">+ Add field</button>
            </div>

            <div class="editor-actions">
              <button class="btn btn--primary" @click="saveProfile">Save</button>
              <button class="btn btn--ghost" @click="cancelEdit">Cancel</button>
            </div>
          </div>

          <button
            v-else-if="profiles.length < maxProfiles"
            class="add-profile-btn"
            @click="startCreate"
          >
            + New profile
          </button>
        </div>

        <div v-if="pendingDeleteId" class="confirm-overlay">
          <div class="confirm-box">
            <p class="confirm-text">Delete this profile?</p>
            <div class="confirm-actions">
              <button class="btn btn--danger" @click="confirmDelete">Delete</button>
              <button class="btn btn--ghost" @click="pendingDeleteId = null">Cancel</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { HolderOutlined, EditOutlined } from '@ant-design/icons-vue';
import type { Profile } from '~/composables/useProfiles';

const TEXTAREA_TOGGLE_LEN = 80;

interface FieldPair {
  key: string;
  value: string;
  textarea: boolean;
}

const props = defineProps<{
  open: boolean;
  profiles: readonly Profile[];
  activeProfile: Profile | null;
  maxProfiles: number;
  defaultFields: Record<string, string>;
}>();

const emit = defineEmits<{
  close: [];
  create: [name: string, fields: Record<string, string>];
  update: [id: string, patch: { name?: string; fields?: Record<string, string> }];
  remove: [id: string];
  setActive: [id: string];
}>();

const editingProfile = ref<Profile | null>(null);
const editName = ref('');
const editFields = ref<FieldPair[]>([]);
const dragIdx = ref(-1);
const dragOverIdx = ref(-1);
const pendingDeleteId = ref<string | null>(null);

function onDragStart(idx: number) {
  dragIdx.value = idx;
}

function onDragOver(idx: number) {
  dragOverIdx.value = idx;
}

function onDrop(targetIdx: number) {
  const from = dragIdx.value;
  if (from === -1 || from === targetIdx) return;
  const item = editFields.value.splice(from, 1)[0];
  editFields.value.splice(targetIdx, 0, item!);
  dragIdx.value = -1;
  dragOverIdx.value = -1;
}

function selectProfile(id: string) {
  emit('setActive', id);
}

function profilePreview(p: Profile): string {
  const entries = Object.entries(p.fields).filter(([, v]) => v);
  if (entries.length === 0) return 'No fields set';
  const priority = new Set(['appearance', 'personality']);
  const sorted = [...entries].sort((a, b) => {
    const aP = priority.has(a[0]) ? 0 : 1;
    const bP = priority.has(b[0]) ? 0 : 1;
    return aP - bP;
  });
  return sorted.slice(0, 2).map(([k, v]) => `${k}: ${v}`).join(', ');
}

function startEdit(profile: Profile) {
  editingProfile.value = profile;
  editName.value = profile.name;
  editFields.value = Object.entries(profile.fields).map(([key, value]) => ({
    key,
    value,
    textarea: value.length > TEXTAREA_TOGGLE_LEN,
  }));
  if (editFields.value.length === 0) addField();
}

function startCreate() {
  const id = '__new__';
  const now = new Date().toISOString();
  editingProfile.value = {
    id,
    name: '',
    fields: { ...props.defaultFields },
    active: false,
    createdAt: now,
    updatedAt: now,
  };
  editName.value = '';
  editFields.value = Object.entries(props.defaultFields).map(([key, value]) => ({
    key,
    value,
    textarea: value.length > TEXTAREA_TOGGLE_LEN,
  }));
}

function addField() {
  editFields.value.push({ key: '', value: '', textarea: false });
}

function removeField(idx: number) {
  editFields.value.splice(idx, 1);
}

function onType(e: KeyboardEvent, idx: number) {
  if (e.ctrlKey || e.altKey) return;
  const value = (e.target as HTMLInputElement | HTMLTextAreaElement).value;
  const pair = editFields.value[idx];
  if (!pair) {
    console.error(`Invalid editField[${idx}]`);
    return;
  }

  const prevTextarea = pair.textarea;
  pair.textarea = value.length > TEXTAREA_TOGGLE_LEN;
  if (prevTextarea !== pair.textarea) {
    nextTick(() => {
      const el = document.querySelectorAll('.field-value-input')[idx] as HTMLInputElement | HTMLTextAreaElement;
      el.focus();
    });
  }
}

function onKeyTab(e: KeyboardEvent, idx: number) {
  if (e.shiftKey) return;
  e.preventDefault();
  nextTick(() => {
    const inputs = document.querySelectorAll<HTMLInputElement>('.fields-list .field-value-input');
    inputs[idx]?.focus();
  });
}

function onKeyEnter(idx: number) {
  nextTick(() => {
    const inputs = document.querySelectorAll<HTMLInputElement>('.fields-list .field-value-input');
    inputs[idx]?.focus();
  });
}

function onValueTab(e: KeyboardEvent, idx: number) {
  const isLast = idx === editFields.value.length - 1;
  if (isLast && editFields.value[idx]!.value.trim() !== '') {
    addField();
  }
}

function onValueEnter(idx: number) {
  const isLast = idx === editFields.value.length - 1;
  if (isLast) {
    saveProfile();
  } else {
    nextTick(() => {
      const inputs = document.querySelectorAll<HTMLInputElement>('.fields-list .field-key-input');
      inputs[idx + 1]?.focus();
    });
  }
}

function buildFieldsFromPairs(): Record<string, string> {
  const fields: Record<string, string> = {};
  for (const pair of editFields.value) {
    const key = pair.key.trim();
    if (!key || !pair.value.trim()) continue;
    fields[key] = pair.value;
  }
  return fields;
}

async function saveProfile() {
  const name = editName.value.trim();
  if (!name) return;

  const fields = buildFieldsFromPairs();
  const isNew = editingProfile.value?.id === '__new__';

  if (isNew) {
    emit('create', name, fields);
  } else if (editingProfile.value) {
    emit('update', editingProfile.value.id, { name, fields });
  }

  editingProfile.value = null;
}

async function removeProfile(id: string) {
  if (editingProfile.value?.id === id) {
    editingProfile.value = null;
  }
  pendingDeleteId.value = null;
  emit('remove', id);
}

function confirmDelete() {
  if (pendingDeleteId.value) removeProfile(pendingDeleteId.value);
}

function cancelEdit() {
  editingProfile.value = null;
}

watch(() => props.open, (val) => {
  if (!val) editingProfile.value = null;
});
</script>

<style scoped>
.confirm-overlay {
  position: absolute;
  inset: 0;
  background: oklch(from var(--gray-9) l c h / 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1;
  border-radius: var(--radius-3);
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

.btn--danger {
  background: var(--red-6);
  color: var(--gray-0);
}

.btn--danger:hover {
  opacity: 0.9;
}

.dialog-overlay {
  position: fixed;
  inset: 0;
  background: oklch(from var(--gray-9) l c h / 0.5);
  z-index: var(--layer-5, 200);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--size-4);
}

.dialog-panel {
  background: var(--surface-2);
  border-radius: var(--radius-3);
  box-shadow: var(--shadow-4);
  inline-size: 100%;
  max-inline-size: var(--size-xl);
  max-block-size: 80dvh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-4) var(--size-5);
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.dialog-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-7);
  margin: 0;
}

.dialog-close {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-5);
  color: var(--text-2);
  padding: var(--size-1);
  line-height: 1;
}

.dialog-close:hover {
  color: var(--text-1);
}

.dialog-body {
  padding: var(--size-5);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
}

.profiles-list {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.profile-card {
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

.profile-card:hover {
  background: var(--surface-3);
}

.profile-card--active {
  border-color: var(--indigo-5);
  background: oklch(from var(--indigo-5) l c h / 0.08);
}

.profile-card-dot {
  inline-size: var(--size-2);
  block-size: var(--size-2);
  border-radius: var(--radius-round);
  background: var(--surface-4);
  flex-shrink: 0;
}

.profile-card--active .profile-card-dot {
  background: var(--indigo-5);
}

.profile-card-info {
  flex: 1;
  min-inline-size: 0;
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
}

.profile-card-name {
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  color: var(--text-1);
}

.profile-card-preview {
  font-size: var(--font-size-0);
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.profile-card-remove {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-2);
  color: var(--text-2);
  padding: var(--size-1);
  line-height: 1;
  flex-shrink: 0;
}

.profile-card-remove:hover {
  color: var(--red-6);
}

.profile-card-edit {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-2);
  color: var(--text-2);
  padding: var(--size-1);
  line-height: 1;
  flex-shrink: 0;
}

.profile-card-edit:hover {
  color: var(--indigo-6);
}

.profile-editor {
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

.fields-list {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.field-row {
  display: flex;
  align-items: flex-start;
  gap: var(--size-1);
  transition: background var(--animation-duration, 0.15s) var(--ease-2);
  border-radius: var(--radius-1);
}

.field-row--drag-over {
  background: oklch(from var(--indigo-5) l c h / 0.1);
}

.field-grip {
  cursor: grab;
  color: var(--text-2);
  font-size: var(--font-size-2);
  line-height: 1;
  flex-shrink: 0;
  padding-block-start: var(--size-2);
  user-select: none;
}

.field-grip:active {
  cursor: grabbing;
}

.field-key-input {
  inline-size: 9rem;
  flex-shrink: 0;
  padding: var(--size-2) var(--size-3);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  background: var(--surface-1);
  color: var(--text-1);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-6);
  font-family: inherit;
  transition: border-color var(--animation-duration, 0.15s) var(--ease-2);
}

.field-key-input:focus {
  outline: none;
  border-color: var(--indigo-6);
}

.field-sep {
  color: var(--text-2);
  flex-shrink: 0;
  font-size: var(--font-size-2);
  padding-block-start: var(--size-2);
}

.field-value-input {
  flex: 1;
  min-inline-size: 0;
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

.field-value-input--expanded {
  resize: vertical;
  min-block-size: var(--size-8);
  line-height: var(--font-lineheight-4);
}

.field-value-input::placeholder,
.field-key-input::placeholder {
  color: var(--text-2);
  opacity: 0.5;
}

.field-remove {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-3);
  color: var(--text-2);
  padding: var(--size-1);
  padding-block-start: var(--size-2);
  line-height: 1;
  flex-shrink: 0;
}

.field-remove:hover {
  color: var(--red-6);
}

.add-field-btn {
  display: inline-block;
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  color: var(--indigo-6);
  padding: var(--size-1) 0;
  font-family: inherit;
}

.add-field-btn:hover {
  text-decoration: underline;
}

.add-profile-btn {
  display: block;
  width: 100%;
  padding: var(--size-3);
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

.add-profile-btn:hover {
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
</style>
