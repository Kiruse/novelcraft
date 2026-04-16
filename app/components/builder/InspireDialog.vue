<template>
  <dialog ref="dialogEl" class="inspire-dialog">
    <div class="dialog-panel">
      <h2 class="dialog-title">Get Inspired</h2>

      <label>
        Keywords or theme
        <textarea
          v-model="keywords"
          rows="3"
          placeholder="e.g. underwater city, time travel, lost civilization…"
          :disabled="loading"
          @keydown.enter.ctrl="generate"
        />
        <span class="hint">Leave empty for random ideas. Ctrl+Enter to generate.</span>
      </label>

      <button type="button" class="btn-go" :disabled="loading" @click.prevent="generate">
        {{ loading ? 'Thinking…' : 'Go' }}
      </button>

      <div v-if="error" class="error">{{ error }}</div>

      <!-- Debug: raw reasoning -->
      <div v-if="reasoning" class="debug-reasoning">
        <button type="button" class="debug-toggle" @click.prevent="showReasoning = !showReasoning">
          {{ showReasoning ? '▾' : '▸' }} Reasoning (debug)
        </button>
        <pre v-if="showReasoning" class="debug-reasoning-content">{{ reasoning }}</pre>
      </div>

      <TransitionGroup name="suggestion-list" tag="div" class="suggestions" v-if="suggestions.length > 0">
        <div v-for="(s, i) in suggestions" :key="i" class="suggestion">
          <h3 class="suggestion-title">{{ s.title || '…' }}</h3>
          <div class="suggestion-meta">
            <code v-if="s.storyId" class="suggestion-slug">{{ s.storyId }}</code>
            <span v-if="s.genre" class="suggestion-genre">{{ s.genre }}</span>
          </div>
          <p class="suggestion-desc">{{ s.description || '…' }}</p>
          <button v-if="completed.has(i)" type="button" class="btn-use" @click.prevent="useSuggestion(s as ParsedSuggestion)">Use</button>
        </div>
      </TransitionGroup>

      <div class="dialog-actions">
        <button type="button" class="dialog-close" @click="close">Close</button>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { SuggestionParser } from '~/utils/suggestionParser';
import type { ParsedSuggestion } from '~/utils/suggestionParser';

const emit = defineEmits<{
  use: [suggestion: ParsedSuggestion];
}>();

const dialogEl = ref<HTMLDialogElement | null>(null);
const keywords = ref('');
const loading = ref(false);
const error = ref('');
const suggestions = ref<Partial<ParsedSuggestion>[]>([]);
const completed = ref<Set<number>>(new Set());
const reasoning = ref('');
const showReasoning = ref(false);

function open() {
  error.value = '';
  suggestions.value = [];
  completed.value = new Set();
  reasoning.value = '';
  showReasoning.value = false;
  dialogEl.value?.showModal();
}

function close() {
  dialogEl.value?.close();
}

async function generate() {
  loading.value = true;
  error.value = '';
  suggestions.value = [];
  completed.value = new Set();
  reasoning.value = '';
  showReasoning.value = false;

  const parser = new SuggestionParser({
    onOpen(index) {
      while (suggestions.value.length <= index) {
        suggestions.value.push({});
      }
    },
    onField(index, field, value) {
      if (index < suggestions.value.length) {
        suggestions.value[index] = { ...suggestions.value[index], [field]: value };
      }
    },
    onComplete(index) {
      completed.value = new Set([...completed.value, index]);
    },
  });

  try {
    const response = await fetch('/api/stories/suggest', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ prompt: keywords.value || undefined }),
    });

    if (!response.ok) {
      error.value = `Failed to generate suggestions (${response.status})`;
      return;
    }

    const reader = response.body?.getReader();
    if (!reader) {
      error.value = 'No response stream';
      return;
    }

    const decoder = new TextDecoder();
    let buffer = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });

      // Parse SSE events (separated by blank lines)
      const parts = buffer.split('\n\n');
      buffer = parts.pop()!;

      for (const part of parts) {
        const lines = part.split('\n');
        const eventType = lines.find(l => l.startsWith('event: '))?.slice(7).trim();
        let data = lines.find(l => l.startsWith('data: '))?.slice(6) ?? '';
        data = data && JSON.parse(data) as string;

        if (eventType === 'error') {
          error.value = data;
          console.error('[inspire] SSE error:', data);
          break;
        }
        if (eventType === 'done') {
          console.log('[inspire] SSE done');
          break;
        }
        if (eventType === 'text') {
          parser.push(data);
        } else if (eventType === 'reasoning') {
          reasoning.value += data;
        } else {
          console.warn('[inspire] unknown SSE event:', eventType, data.slice(0, 100));
        }
      }
    }

    parser.finish();
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : 'Failed to generate suggestions';
  } finally {
    loading.value = false;
  }
}

function useSuggestion(s: ParsedSuggestion) {
  emit('use', s);
  close();
}

defineExpose({ open });
</script>

<style scoped>
.inspire-dialog {
  border: none;
  padding: 0;
  background: none;
  max-inline-size: var(--size-content-3);
  inline-size: 100%;
}

.inspire-dialog::backdrop {
  background: oklch(0 0 0 / 0.4);
}

.dialog-panel {
  background: var(--surface-1);
  border-radius: var(--radius-3);
  box-shadow: var(--shadow-6);
  padding: var(--size-6);
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
}

.dialog-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-6);
}

label {
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  color: var(--text-2);
}

.hint {
  font-size: var(--font-size-0);
  font-style: italic;
  color: var(--text-2);
}

textarea {
  resize: vertical;
  min-block-size: 3lh;
}

.btn-go {
  align-self: flex-start;
  padding: var(--size-2) var(--size-6);
  background: var(--brand-gradient);
  color: var(--gray-0);
  border: none;
  border-radius: var(--radius-2);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-6);
  cursor: pointer;
}

.btn-go:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error {
  color: var(--red-6);
  font-size: var(--font-size-1);
}

/* Debug reasoning */
.debug-reasoning {
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
}

.debug-toggle {
  background: none;
  border: none;
  padding: 0;
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-5);
  color: var(--text-2);
  cursor: pointer;
  text-align: start;
}

.debug-reasoning-content {
  font-size: var(--font-size-0);
  color: var(--text-2);
  background: var(--surface-2);
  border-radius: var(--radius-1);
  padding: var(--size-3);
  margin: 0;
  max-block-size: 15lh;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-word;
}

/* Suggestions */
.suggestions {
  display: flex;
  flex-direction: column;
  gap: var(--size-3);
  max-block-size: 50vh;
  overflow-y: auto;
  padding-inline-end: var(--size-2);
}

.suggestion {
  background: var(--surface-2);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  padding: var(--size-4);
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.suggestion-title {
  font-size: var(--font-size-3);
  font-weight: var(--font-weight-6);
  color: var(--text-1);
  margin: 0;
}

.suggestion-genre {
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-5);
  color: var(--text-2);
}

.suggestion-meta {
  display: flex;
  align-items: center;
  gap: var(--size-2);
}

.suggestion-slug {
  font-size: var(--font-size-0);
  color: var(--indigo-6);
  background: var(--indigo-0);
  padding: var(--size-1) var(--size-2);
  border-radius: var(--radius-1);
}

.suggestion-desc {
  font-size: var(--font-size-1);
  color: var(--text-2);
  line-height: var(--font-lineheight-4);
  margin: 0;
}

.btn-use {
  align-self: flex-start;
  padding: var(--size-1) var(--size-4);
  background: var(--surface-3);
  color: var(--text-1);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-1);
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-6);
  cursor: pointer;
}

.btn-use:hover {
  background: var(--surface-4);
}

/* Transition group animation */
.suggestion-list-enter-active {
  transition: all 0.3s ease-out;
}
.suggestion-list-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
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
</style>
