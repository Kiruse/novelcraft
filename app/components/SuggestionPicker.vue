<template>
  <div class="suggestion-picker">
    <div v-if="error" class="suggestion-error">{{ error }}</div>

    <!-- Collapsible reasoning -->
    <Collapsible v-if="reasoning" v-model="showReasoning" class="thoughts-collapsible">
      <template #header>Thoughts</template>
      <pre class="reasoning-content">{{ reasoning }}</pre>
    </Collapsible>

    <!-- Suggestion cards -->
    <TransitionGroup name="suggestion-list" tag="div" class="suggestions">
      <div
        v-for="(s, i) in suggestions"
        :key="i"
        class="suggestion-card"
        :class="{
          'suggestion-card--complete': completed.has(i),
          'suggestion-card--selected': i === selectedIndex,
        }"
      >
        <h4 class="suggestion-card-title">{{ s.title || '…' }}</h4>
        <span v-if="s.genre" class="suggestion-card-genre">{{ s.genre }}</span>
        <p class="suggestion-card-desc">{{ s.description || '…' }}</p>
        <button
          v-if="completed.has(i)"
          class="suggestion-card-use"
          @click="$emit('select', suggestions[i] as ParsedSuggestion)"
        >
          Use this
        </button>
      </div>
    </TransitionGroup>

    <div v-if="loading && suggestions.length === 0" class="suggestion-loading">
      Thinking…
    </div>
  </div>
</template>

<script setup lang="ts">
import { SuggestionParser } from '~/utils/suggestionParser';
import type { ParsedSuggestion } from '~/utils/suggestionParser';
import Collapsible from '~/components/Collapsible.vue';

export interface SuggestionPickerSuggestion extends ParsedSuggestion {}

const props = defineProps<{
  /** URL to POST to for SSE stream */
  endpoint: string;
  /** Request body sent as JSON */
  body?: Record<string, unknown>;
  /** Tag names to parse inside <suggestion>. Defaults to title, genre, description. */
  knownTags?: string[];
  /** Whether a generation request is currently in-flight. */
  loading: boolean;
  /** Parsed suggestions so far. */
  suggestions: Partial<ParsedSuggestion>[];
  /** Set of completed suggestion indices. */
  completed: Set<number>;
  /** Accumulated reasoning text. */
  reasoning: string;
  /** Index of the currently selected suggestion, or null. */
  selectedIndex: number | null;
  /** Error message, if any. */
  error: string;
}>();

const emit = defineEmits<{
  select: [suggestion: ParsedSuggestion];
  'update:suggestions': [suggestions: Partial<ParsedSuggestion>[]];
  'update:completed': [completed: Set<number>];
  'update:reasoning': [reasoning: string];
  'update:error': [error: string];
  done: [];
}>();

const showReasoning = ref(false);

function clear() {
  emit('update:suggestions', []);
  emit('update:completed', new Set());
  emit('update:reasoning', '');
  emit('update:error', '');
  showReasoning.value = false;
}

async function generate() {
  clear();

  const tags = new Set(props.knownTags ?? ['title', 'genre', 'description']);

  const parser = new SuggestionParser({
    knownTags: tags,
    onOpen(index: number) {
      const updated = [...props.suggestions];
      while (updated.length <= index) updated.push({});
      emit('update:suggestions', updated);
    },
    onField(index: number, field: string, value: string) {
      if (index < props.suggestions.length) {
        const updated = [...props.suggestions];
        updated[index] = { ...updated[index], [field]: value };
        emit('update:suggestions', updated);
      }
    },
    onComplete(index: number) {
      emit('update:completed', new Set([...props.completed, index]));
    },
  });

  try {
    const response = await fetch(props.endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: props.body ? JSON.stringify(props.body) : undefined,
    });

    if (!response.ok) {
      emit('update:error', `Failed to generate suggestions (${response.status})`);
      return;
    }

    const reader = response.body?.getReader();
    if (!reader) {
      emit('update:error', 'No response stream');
      return;
    }

    const decoder = new TextDecoder();
    let buffer = '';
    let localReasoning = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });

      const parts = buffer.split('\n\n');
      buffer = parts.pop()!;

      for (const part of parts) {
        const lines = part.split('\n');
        const eventType = lines.find(l => l.startsWith('event: '))?.slice(7).trim();
        const dataLine = lines.find(l => l.startsWith('data: '))?.slice(6) ?? '';
        let data = '';
        if (dataLine) {
          try { data = JSON.parse(dataLine) as string; } catch { data = dataLine; }
        }

        if (eventType === 'error') {
          emit('update:error', data);
          break;
        }
        if (eventType === 'done') break;
        if (eventType === 'text') {
          parser.push(data);
        } else if (eventType === 'reasoning') {
          localReasoning += data;
          emit('update:reasoning', localReasoning);
        }
      }
    }

    parser.finish();
  } catch (e: unknown) {
    emit('update:error', e instanceof Error ? e.message : 'Failed to generate suggestions');
  } finally {
    emit('done');
  }
}

defineExpose({ generate, clear });
</script>

<style scoped>
.suggestion-picker {
  display: flex;
  flex-direction: column;
  gap: var(--size-4);
}

.suggestion-error {
  color: var(--red-6);
  font-size: var(--font-size-2);
}

.thoughts-collapsible {
  --collapsible-header-padding: var(--size-2) var(--size-3);
  --collapsible-body-padding: 0 var(--size-3) var(--size-3);
}

.reasoning-content {
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

.suggestions {
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  gap: var(--size-4);
}

.suggestion-card {
  inline-size: var(--size-content-3);
  flex-shrink: 0;
  background: var(--surface-2);

  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-3);
  padding: var(--size-5);
  display: flex;
  flex-direction: column;
  gap: var(--size-3);
  transition: border-color var(--animation-duration, 0.15s) var(--ease-2),
    box-shadow var(--animation-duration, 0.15s) var(--ease-2);
}

@media (max-width: 767px) {
  .suggestion-card {
    inline-size: var(--size-content-2);
  }
}

.suggestion-card--complete {
  border-color: var(--indigo-4);
}

.suggestion-card--selected {
  border-color: var(--green-4);
  box-shadow: 0 0 0 var(--border-size-1) oklch(from var(--green-6) l c h / 0.3);
}

.suggestion-card-title {
  font-size: var(--font-size-3);
  font-weight: var(--font-weight-6);
  color: var(--text-1);
  margin: 0;
}

.suggestion-card-genre {
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-5);
  color: var(--text-2);
  align-self: flex-start;
  background: var(--surface-3);
  padding: var(--size-1) var(--size-2);
  border-radius: var(--radius-1);
}

.suggestion-card-desc {
  font-size: var(--font-size-2);
  color: var(--text-2);
  line-height: var(--font-lineheight-4);
  margin: 0;
}

.suggestion-card-use {
  align-self: flex-start;
  padding: var(--size-2) var(--size-5);
  background: var(--brand-gradient);
  color: var(--gray-0);
  border: none;
  border-radius: var(--radius-2);
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  transition: transform var(--animation-duration, 0.15s) var(--ease-2),
    box-shadow var(--animation-duration, 0.15s) var(--ease-2);
}

.suggestion-card-use:hover {
  transform: translateY(calc(var(--size-1) * -1));
  box-shadow: var(--shadow-3);
}

.suggestion-loading {
  color: var(--text-2);
  font-style: italic;
  font-size: var(--font-size-2);
}

/* Transition group animation */
.suggestion-list-enter-active {
  transition: all 0.3s ease-out;
}

.suggestion-list-enter-from {
  opacity: 0;
  transform: translateY(10px);
}
</style>
