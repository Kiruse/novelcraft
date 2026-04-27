<template>
  <div class="game">
    <!-- Title bar -->
    <div class="game-header">
      <NuxtLink to="/" class="game-back">← Back</NuxtLink>
      <input
        :value="title"
        class="game-title-input"
        :placeholder="titlePlaceholder"
        @blur="emit('updateTitle', ($event.target as HTMLInputElement).value)"
        @keydown.enter.prevent="($event.target as HTMLInputElement).blur()"
      />
    </div>

    <!-- Story pages -->
    <div class="game-body">
      <Transition name="page" mode="out-in">
        <div v-if="currentEntry" :key="'page-' + currentEntry.id" class="story-page" :class="{ 'story-page--editing': editing }">
          <!-- System prompt (steer notes) -->
          <div v-if="currentEntry.system && !editingSystem" class="story-system story-system--editable" @click="startSystemEditing">
            {{ currentEntry.system }}
          </div>
          <textarea
            v-if="editingSystem"
            ref="editSystemArea"
            v-model="editSystemText"
            class="story-system-editor"
            placeholder="Steer notes (clear to remove)…"
            @blur="finishSystemEditing"
            @keydown.escape.prevent="finishSystemEditing"
          />

          <!-- User prompt (quoted) -->
          <blockquote v-if="currentEntry.prompt" class="story-quote">
            {{ currentEntry.prompt }}
          </blockquote>

          <!-- Streaming response -->
          <div v-if="streaming" class="story-prose">
            <p v-if="streamText" class="streaming-paragraph">{{ normalizeContent(streamText) }}<span class="streaming-cursor" /></p>
            <p v-else class="streaming-typing">Thinking…</p>
          </div>

          <!-- Completed response -->
          <template v-else>
            <!-- Edit mode -->
            <textarea
              v-if="editing"
              ref="editArea"
              v-model="editText"
              class="story-prose-editor"
              @blur="finishEditing"
              @keydown.escape.prevent="finishEditing"
              @keydown.ctrl.enter.prevent="finishEditing"
            />
            <!-- Rendered markdown (click to edit) -->
            <div
              v-else-if="currentEntry.response"
              class="story-prose story-prose--editable"
              @click="startEditing"
              v-html="renderedResponse"
            />
          </template>
        </div>
      </Transition>
    </div>

    <!-- Navigation + input -->
    <div class="game-controls">
      <div class="page-nav">
        <button
          class="page-nav-btn"
          :disabled="currentPage <= 0 || streaming"
          @click="setPage(currentPage - 1)"
        >
          ←
        </button>
        <span class="page-indicator">{{ pages.length > 0 ? currentPage + 1 : 0 }} / {{ pages.length }}</span>
        <button
          class="page-nav-btn"
          :disabled="currentPage >= pages.length - 1 || streaming"
          @click="setPage(currentPage + 1)"
        >
          →
        </button>
      </div>

      <form class="chat-input" @submit.prevent="onSubmit">
        <button
          type="button"
          class="mode-btn"
          :title="modeHint"
          @click="cycleMode"
        >
          <template v-if="mode === 'steer'">Steer /<br />Remind</template>
          <template v-else>{{ modeLabel }}</template>
        </button>
        <input
          ref="chatField"
          v-model="input"
          type="text"
          :placeholder="modePlaceholder"
          :disabled="streaming"
          class="chat-input-field"
          autofocus
          @keydown.tab="onTab"
          @keydown.arrow-up="onArrowUp"
        />
        <button type="submit" :disabled="streaming || (!input.trim() && mode !== 'write')" class="chat-input-send">
          <template v-if="!input.trim() && mode === 'write'">Write more</template>
          <template v-else>Send</template>
        </button>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { GamePage } from '~/utils/msgUtils';
import { renderMarkdown, normalizeContent } from '~/utils/msgUtils';

export type InputMode = 'write' | 'steer' | 'instruct';

const MODE_ORDER: InputMode[] = ['write', 'steer', 'instruct'];

const MODE_LABELS: Record<InputMode, string> = {
  write: 'Write',
  steer: 'Steer / Remind',
  instruct: 'Instruct',
};

const MODE_HINTS: Record<InputMode, string> = {
  write: 'Start a new page (Shift+Tab to switch)',
  steer: 'Nudge the current page\'s direction (Shift+Tab to switch)',
  instruct: 'Regenerate with instructions (Shift+Tab to switch)',
};

const MODE_PLACEHOLDERS: Record<InputMode, string> = {
  write: 'What do you do?',
  steer: 'How should this page change?',
  instruct: 'How should this be rewritten?',
};

const SLASH_MODE_COMMANDS: Record<string, InputMode> = {
  steer: 'steer',
  remind: 'steer',
  write: 'write',
  instruct: 'instruct',
};

const props = withDefaults(defineProps<{
  /** Pre-built page entries. */
  pages: GamePage[];
  /** Story title shown in the header. */
  title?: string;
  /** Placeholder for the title input. */
  titlePlaceholder?: string;
  /** Whether the agent is currently streaming. */
  streaming?: boolean;
  /** Accumulated streaming text. */
  streamText?: string;
}>(), {
  title: '',
  titlePlaceholder: 'Untitled',
  streaming: false,
  streamText: '',
});

const emit = defineEmits<{
  /** Emitted when the user submits a message. */
  prompt: [payload: { text: string; mode: InputMode; pageId: number | string | null }];
  /** Emitted when the title input is blurred with a new value. */
  updateTitle: [title: string];
  /** Emitted when the user edits a page's response. */
  updatePage: [payload: { pageId: number | string; response?: string; system?: string | null }];
}>();

const currentPage = ref(props.pages.length - 1);
const input = ref('');
const mode = ref<InputMode>('write');

// --- Response editing state ---
const editing = ref(false);
const editText = ref('');
const editArea = ref<HTMLTextAreaElement | null>(null);

// --- System editing state ---
const editingSystem = ref(false);
const editSystemText = ref('');
const editSystemArea = ref<HTMLTextAreaElement | null>(null);
const chatField = ref<HTMLInputElement | null>(null);

const currentEntry = computed(() =>
  currentPage.value >= 0 && currentPage.value < props.pages.length
    ? props.pages[currentPage.value]!
    : null,
);

const renderedResponse = computed(() => {
  const response = currentEntry.value?.response;
  if (!response) return '';
  return renderMarkdown(response.trim());
});

const modeLabel = computed(() => MODE_LABELS[mode.value]);
const modeHint = computed(() => MODE_HINTS[mode.value]);
const modePlaceholder = computed(() => MODE_PLACEHOLDERS[mode.value]);

// Auto-advance to new last page when on previous last page
watch(() => props.pages.length, (len) => {
  if (currentPage.value === len - 2)
    currentPage.value = len - 1;
});

// Cancel editing when navigating away from the current page
watch(currentPage, () => {
  editing.value = false;
  editingSystem.value = false;
});

// --- Slash command detection ---
watch(input, (val, oldVal) => {
  // Only check when the user just typed a space after a slash command
  if (val.length <= oldVal.length) return;

  const match = val.match(/^\/(steer|remind|write|instruct)\s(.*)/);
  if (match) {
    const command = match[1]!;
    const rest = match[2] ?? '';
    const newMode = SLASH_MODE_COMMANDS[command]!;
    mode.value = newMode;
    input.value = rest;
  }
});

function onSubmit() {
  const rawText = input.value.trim();
  if (!rawText && mode.value !== 'write') return;
  if (props.streaming) return;

  // Bare slash command without content — just switch mode
  const bareMatch = rawText.match(/^\/(steer|remind|write|instruct)$/);
  if (bareMatch) {
    mode.value = SLASH_MODE_COMMANDS[bareMatch[1]!]!;
    input.value = '';
    return;
  }

  input.value = '';
  emit('prompt', {
    text: rawText,
    mode: mode.value,
    pageId: currentEntry.value?.id ?? null,
  });
}

function onTab(e: KeyboardEvent) {
  if (e.shiftKey) {
    e.preventDefault();
    cycleMode();
  }
}

function onArrowUp() {
  if (input.value === '' && currentEntry.value?.response && !props.streaming) {
    startEditing();
  }
}

function cycleMode() {
  const idx = MODE_ORDER.indexOf(mode.value);
  mode.value = MODE_ORDER[(idx + 1) % MODE_ORDER.length]!;
}

function setPage(n: number) {
  currentPage.value = Math.max(0, Math.min(props.pages.length - 1, n));
}

// --- Editing ---

function startEditing() {
  if (!currentEntry.value?.response) return;
  editText.value = currentEntry.value.response;
  editing.value = true;
  nextTick(() => editArea.value?.focus());
}

function finishEditing() {
  if (!editing.value) return;
  editing.value = false;
  const entry = currentEntry.value;
  if (entry && editText.value !== entry.response) {
    emit('updatePage', { pageId: entry.id, response: editText.value });
  }
  nextTick(() => chatField.value?.focus());
}

// --- System editing ---

function startSystemEditing() {
  const entry = currentEntry.value;
  if (!entry) return;
  editSystemText.value = entry.system ?? '';
  editingSystem.value = true;
  nextTick(() => editSystemArea.value?.focus());
}

function finishSystemEditing() {
  if (!editingSystem.value) return;
  editingSystem.value = false;
  const entry = currentEntry.value;
  if (!entry) return;
  const newValue = editSystemText.value.trim() || null;
  if (newValue !== entry.system) {
    emit('updatePage', { pageId: entry.id, system: newValue });
  }
}

defineExpose({ setPage });
</script>

<style scoped>
.game {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  block-size: 100%;
}

/* --- Title bar --- */

.game-header {
  display: flex;
  align-items: center;
  gap: var(--size-4);
  padding: var(--size-4) var(--size-6);
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.game-back {
  font-size: var(--font-size-2);
  color: var(--text-2);
  text-decoration: none;
}

.game-back:hover {
  color: var(--text-1);
}

.game-title-input {
  background: none;
  border: var(--border-size-1) solid transparent;
  border-radius: var(--radius-2);
  padding: var(--size-1) var(--size-2);
  color: var(--text-1);
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-6);
  transition: border-color var(--animation-duration, 0.15s) var(--ease-2),
    background var(--animation-duration, 0.15s) var(--ease-2);
}

.game-title-input:hover {
  border-color: var(--surface-4);
}

.game-title-input:focus {
  outline: none;
  border-color: var(--indigo-6);
  background: var(--surface-2);
}

.game-title-input::placeholder {
  color: var(--text-2);
  opacity: 0.5;
}

/* --- Story body --- */

.game-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--size-6) 0;
  display: flex;
  flex-direction: column;
}

.story-page {
  max-inline-size: var(--size-content-3);
  margin-inline: auto;
  inline-size: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--size-5);
}

/* --- System prompt callout --- */

.story-system {
  padding: var(--size-3) var(--size-4);
  border-inline-start: 2px solid var(--surface-2);
  background: rgba(0, 0, 0, 0.2);
  font-size: var(--font-size-1);
  line-height: var(--font-lineheight-4);
  color: var(--text-3);
  white-space: pre-wrap;
  word-break: break-word;
}

.story-system--editable {
  cursor: text;
  border-radius: var(--radius-2);
  transition: background var(--animation-duration, 0.15s) var(--ease-2),
    box-shadow var(--animation-duration, 0.15s) var(--ease-2);
}

.story-system--editable:hover {
  background: rgba(0, 0, 0, 0.3);
  box-shadow: 0 0 0 var(--border-size-1) var(--surface-4);
}

.story-system-editor {
  inline-size: 100%;
  padding: var(--size-3) var(--size-4);
  border-inline-start: 2px solid var(--surface-2);
  background: rgba(0, 0, 0, 0.2);
  font-size: var(--font-size-1);
  line-height: var(--font-lineheight-4);
  color: var(--text-3);
  font-family: inherit;
  resize: vertical;
  field-sizing: content;
}

.story-system-editor:focus {
  outline: none;
}

.story-system-editor::placeholder {
  color: var(--text-4);
}

.story-quote {
  margin: 0;
  padding: var(--size-3) var(--size-4);
  border-inline-start: var(--border-size-3) solid var(--indigo-4);
  background: var(--surface-2);
  border-radius: var(--radius-2);
  font-style: italic;
  color: var(--text-2);
  font-size: var(--font-size-2);
  line-height: var(--font-lineheight-4);
  white-space: pre-wrap;
  word-break: break-word;
}

.story-prose {
  font-size: var(--font-size-2);
  line-height: var(--font-lineheight-4);
  color: var(--text-1);
  word-break: break-word;
}

/* --- Editable prose --- */

.story-prose--editable {
  cursor: text;
  border-radius: var(--radius-2);
  transition: background var(--animation-duration, 0.15s) var(--ease-2),
    box-shadow var(--animation-duration, 0.15s) var(--ease-2);
}

.story-prose--editable:hover {
  background: var(--surface-2);
  box-shadow: 0 0 0 var(--border-size-1) var(--surface-4);
}

/* --- Editing mode: page fills the body --- */

.story-page--editing {
  flex: 1;
  min-block-size: 0;
  overflow: hidden;
}

.story-page--editing .story-prose-editor {
  flex: 1;
  min-block-size: 0;
  resize: none;
}

/* --- Markdown content styles --- */

.story-prose :deep(p) {
  margin-block-end: var(--size-3);
}

.story-prose :deep(p:last-child) {
  margin-block-end: 0;
}

.story-prose :deep(strong) {
  font-weight: var(--font-weight-7);
}

.story-prose :deep(em) {
  font-style: italic;
}

.story-prose :deep(h1),
.story-prose :deep(h2),
.story-prose :deep(h3) {
  font-weight: var(--font-weight-7);
  margin-block-start: var(--size-4);
  margin-block-end: var(--size-2);
}

.story-prose :deep(blockquote) {
  border-inline-start: var(--border-size-2) solid var(--surface-4);
  padding-inline-start: var(--size-3);
  color: var(--text-2);
  margin-block: var(--size-3);
}

.story-prose :deep(ul),
.story-prose :deep(ol) {
  padding-inline-start: var(--size-5);
  margin-block: var(--size-2);
}

.story-prose :deep(li) {
  margin-block-end: var(--size-1);
}

.story-prose :deep(code) {
  font-family: var(--font-mono, monospace);
  font-size: 0.9em;
  background: var(--surface-3);
  padding: var(--size-1) var(--size-2);
  border-radius: var(--radius-1);
}

.story-prose :deep(pre) {
  background: var(--surface-2);
  padding: var(--size-3);
  border-radius: var(--radius-2);
  overflow-x: auto;
  margin-block: var(--size-3);
}

.story-prose :deep(pre code) {
  background: none;
  padding: 0;
}

.story-prose :deep(hr) {
  border: none;
  border-block-start: var(--border-size-1) solid var(--surface-3);
  margin-block: var(--size-4);
}

.story-prose :deep(a) {
  color: var(--indigo-6);
  text-decoration: underline;
}

/* --- Prose editor (textarea) --- */

.story-prose-editor {
  inline-size: 100%;
  border: none;
  background: transparent;
  color: var(--text-1);
  font-family: inherit;
  font-size: var(--font-size-2);
  line-height: var(--font-lineheight-4);
  padding: 0;
  resize: vertical;
}

.story-prose-editor:focus {
  outline: none;
}

/* --- Streaming --- */

.streaming-paragraph {
  font-size: var(--font-size-2);
  line-height: var(--font-lineheight-4);
  color: var(--text-2);
  white-space: pre-wrap;
  word-break: break-word;
  margin: 0;
}

.streaming-cursor {
  display: inline-block;
  inline-size: 2px;
  block-size: 1lh;
  background: var(--indigo-6);
  margin-inline-start: var(--size-1);
  vertical-align: text-bottom;
  animation: cursor-blink 1s step-end infinite;
}

@keyframes cursor-blink {
  50% { opacity: 0; }
}

.streaming-typing {
  font-style: italic;
  color: var(--text-2);
  opacity: 0.7;
  font-size: var(--font-size-2);
  margin: 0;
}

/* --- Page transition --- */

.page-enter-active,
.page-leave-active {
  transition: opacity var(--animation-duration, 0.15s) var(--ease-2);
}

.page-enter-from,
.page-leave-to {
  opacity: 0;
}

/* --- Controls (nav + input) --- */

.game-controls {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
  padding: var(--size-3) var(--size-6) var(--size-4);
  border-block-start: var(--border-size-1) solid var(--surface-3);
}

.page-nav {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--size-4);
}

.page-nav-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 25px;
  height: 25px;
  background: var(--surface-3);
  color: var(--text-1);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: 50%;
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  cursor: pointer;
  transition: background var(--animation-duration, 0.15s) var(--ease-2);
}

.page-nav-btn:hover:not(:disabled) {
  background: var(--surface-4);
}

.page-nav-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.page-indicator {
  font-size: var(--font-size-0);
  color: var(--text-2);
  font-variant-numeric: tabular-nums;
  min-inline-size: var(--size-10);
  text-align: center;
}

/* --- Input bar --- */

.chat-input {
  display: flex;
  gap: var(--size-2);
}

/* --- Mode button --- */

.mode-btn {
  padding: var(--size-2) var(--size-3);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  background: var(--surface-2);
  color: var(--text-2);
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  line-height: 1.2;
  text-align: center;
  inline-size: 7em;
  transition: background var(--animation-duration, 0.15s) var(--ease-2),
    border-color var(--animation-duration, 0.15s) var(--ease-2),
    color var(--animation-duration, 0.15s) var(--ease-2);
}

.mode-btn:hover {
  background: var(--surface-3);
  color: var(--text-1);
}

.chat-input-field {
  flex: 1;
  border-color: var(--surface-4);
  border-radius: var(--radius-2);
}

.chat-input-field:focus {
  border-color: var(--indigo-6);
}

.chat-input-field:disabled {
  opacity: 0.6;
}

.chat-input-send {
  padding: var(--size-3) var(--size-6);
  background: var(--brand-gradient);
  color: var(--gray-0);
  border: none;
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
}

.chat-input-send:hover:not(:disabled) {
  transform: translateY(calc(var(--size-px-1) * -1));
}

.chat-input-send:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
