<template>
  <div v-if="!vignette" class="vignette-loading">Loading...</div>
  <div v-else class="vignette-page">
    <!-- Draft / Compose mode (no game session yet) -->
    <template v-if="!activeSessionId">
      <div class="vignette-compose">
        <input
          v-model="vignetteTitle"
          class="vignette-title-input"
          placeholder="Untitled Vignette"
          @blur="saveTitle"
          @keydown.enter.prevent="($event.target as HTMLInputElement).blur()"
        />
        <p class="vignette-hint">
          Write a disposition — a few keywords, a theme, or a full paragraph describing the story you want to experience.
          Or leave it empty and let us surprise you.
        </p>

        <textarea
          v-model="disposition"
          class="disposition-area"
          placeholder="A lonely lighthouse keeper on a storm-wracked island starts receiving radio messages from a ship that sank fifty years ago..."
          rows="6"
          @input="onDispositionInput"
        />

        <div class="compose-actions">
          <button
            class="btn btn--secondary"
            :disabled="suggesting"
            @click="requestSuggestion"
          >
            {{ suggesting ? 'Generating...' : 'Suggest something' }}
          </button>
          <button
            class="btn btn--primary"
            :disabled="!canLockIn"
            @click="lockIn"
          >
            Lock in
          </button>
        </div>

        <!-- Suggestion picker -->
        <SuggestionPicker
          v-if="showPicker"
          ref="pickerRef"
          endpoint="/api/vignettes/suggest"
          :body="{ prompt: disposition }"
          :known-tags="['title', 'genre', 'description']"
          :loading="suggesting"
          :suggestions="suggestions"
          :completed="completedSet"
          :selected-index="selectedIndex"
          :reasoning="reasoning"
          :error="suggestionError"
          @select="useSuggestion"
          @update:suggestions="suggestions = $event"
          @update:completed="completedSet = $event"
          @update:reasoning="reasoning = $event"
          @update:error="suggestionError = $event"
          @done="suggesting = false"
        />
      </div>
    </template>

    <!-- Playing mode (session active) -->
    <template v-else>
      <div class="vignette-game">
        <div class="game-header">
          <NuxtLink to="/" class="game-back">← Back</NuxtLink>
          <input
            v-model="vignetteTitle"
            class="game-title game-title--input"
            placeholder="Untitled Vignette"
            @blur="saveTitle"
            @keydown.enter.prevent="($event.target as HTMLInputElement).blur()"
          />
        </div>

        <div class="chat-area" ref="chatArea">
          <div
            v-for="msg in messages"
            :key="msg.id"
            :class="['chat-message', `chat-message--${msg.role}`]"
          >
            <div class="chat-message-content">{{ msg.contents }}</div>
          </div>
          <div v-if="streaming" class="chat-message chat-message--agent">
            <div class="chat-message-content">
              <span v-if="streamText">{{ streamText }}</span>
              <span v-else class="typing">Thinking...</span>
            </div>
          </div>
        </div>

        <form class="chat-input" @submit.prevent="sendMessage">
          <input
            v-model="input"
            type="text"
            placeholder="What do you do?"
            :disabled="streaming"
            class="chat-input-field"
            autofocus
          />
          <button type="submit" :disabled="streaming || !input.trim()" class="chat-input-send">
            Send
          </button>
        </form>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import type { VignetteShape } from '~/composables/useCurrentUser';
import type { ParsedSuggestion } from '~/utils/suggestionParser';

interface VignetteMessageShape {
  id: number;
  gameSessionId: number;
  role: 'user' | 'agent' | 'system';
  contents: string;
  createdAt: string;
}

const route = useRoute();
const id = computed(() => route.params.id as string);

const { data, refresh: refreshVignette } = await useFetch<{
  vignette: VignetteShape;
  session: { id: number } | null;
  messages: VignetteMessageShape[];
}>(`/api/vignettes/${id.value}`);

const vignette = computed(() => data.value?.vignette);
const activeSessionId = computed(() => data.value?.session?.id ?? null);
const messages = ref<VignetteMessageShape[]>([]);

// Initialize messages from server data
watch(() => data.value?.messages, (msgs) => {
  if (msgs) messages.value = [...msgs];
}, { immediate: true });

// --- Draft mode ---
const disposition = ref('');
const vignetteTitle = ref('');
const suggesting = ref(false);
const showPicker = ref(false);
const pickerRef = ref<{ generate: () => void } | null>(null);
const suggestions = ref<Partial<ParsedSuggestion>[]>([]);
const completedSet = ref<Set<number>>(new Set());
const reasoning = ref('');
const suggestionError = ref('');
const selectedSuggestion = ref<ParsedSuggestion | null>(null);
const selectedIndex = ref<number | null>(null);

// Initialize disposition and title from story
watch(() => data.value?.vignette, (v) => {
  if (v) {
    disposition.value = v.description ?? '';
    vignetteTitle.value = v.title ?? '';
  }
}, { immediate: true });

// Clear suggestion selection when user edits the textarea
let programmaticDisposition = false;

function onDispositionInput() {
  if (!programmaticDisposition && selectedIndex.value !== null) {
    selectedSuggestion.value = null;
    selectedIndex.value = null;
  }
}

const { refresh: refreshUserData } = useCurrentUser();

async function saveTitle() {
  const newTitle = vignetteTitle.value.trim() || 'Untitled Vignette';
  vignetteTitle.value = newTitle;
  try {
    await $fetch(`/api/vignettes/${id.value}`, {
      method: 'PATCH',
      body: { title: newTitle },
    });
    await refreshVignette();
    await refreshUserData();
  } catch (e) {
    console.error('Failed to save title', e);
  }
}

const canLockIn = computed(() => {
  return disposition.value.trim().length > 0 || selectedIndex.value !== null;
});

function requestSuggestion() {
  if (!showPicker.value) {
    showPicker.value = true;
    suggesting.value = true;
    nextTick(() => pickerRef.value?.generate());
  } else {
    suggesting.value = true;
    pickerRef.value?.generate();
  }
}

function useSuggestion(suggestion: ParsedSuggestion) {
  // Find the index of this suggestion in the array
  const idx = suggestions.value.findIndex(
    s => s.title === suggestion.title && s.description === suggestion.description,
  );
  if (selectedIndex.value === idx) {
    // Deselect
    selectedSuggestion.value = null;
    selectedIndex.value = null;
  } else {
    selectedSuggestion.value = suggestion;
    selectedIndex.value = idx;
    if (suggestion.description) {
      programmaticDisposition = true;
      disposition.value = suggestion.description;
      nextTick(() => { programmaticDisposition = false; });
    }
  }
}

async function lockIn() {
  const patchBody: Record<string, string> = {};
  if (disposition.value.trim()) patchBody.disposition = disposition.value;
  if (selectedSuggestion.value) {
    if (selectedSuggestion.value.title) patchBody.title = selectedSuggestion.value.title;
    if (selectedSuggestion.value.description) patchBody.premise = selectedSuggestion.value.description;
  }

  if (Object.keys(patchBody).length > 0) {
    await $fetch(`/api/vignettes/${id.value}`, {
      method: 'PATCH',
      body: patchBody,
    });
  }

  // Start the game via SSE — creates a session and streams opening narration
  streaming.value = true;
  streamText.value = '';

  try {
    const response = await fetch(`/api/vignettes/${id.value}/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    });

    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    const fullText = await consumeTextStream(response);
    void fullText;

    await refreshVignette();
  } catch (err) {
    console.error('Start failed:', err);
  } finally {
    streaming.value = false;
    streamText.value = '';
  }
}

// --- Play mode ---
const streaming = ref(false);
const streamText = ref('');
const input = ref('');
const chatArea = ref<HTMLElement | null>(null);

/** Consume an SSE stream, only accumulating 'text' events into streamText. */
async function consumeTextStream(response: Response): Promise<string> {
  const reader = response.body?.getReader();
  if (!reader) throw new Error('No response body');

  const decoder = new TextDecoder();
  let buffer = '';
  let fullText = '';

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

      if (eventType === 'text') {
        fullText += data;
        streamText.value += data;
      }
    }
  }

  return fullText;
}

async function sendMessage() {
  const text = input.value.trim();
  if (!text || streaming.value || !activeSessionId.value) return;

  // Optimistically add user message
  messages.value.push({
    id: Date.now(),
    gameSessionId: activeSessionId.value,
    role: 'user',
    contents: text,
    createdAt: new Date().toISOString(),
  });
  input.value = '';

  await nextTick();
  scrollToBottom();

  streaming.value = true;
  streamText.value = '';

  try {
    const response = await fetch(`/api/vignettes/${id.value}/message`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ content: text, sessionId: activeSessionId.value }),
    });

    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    const fullText = await consumeTextStream(response);

    if (fullText.trim()) {
      messages.value.push({
        id: Date.now() + 1,
        gameSessionId: activeSessionId.value,
        role: 'agent',
        contents: fullText,
        createdAt: new Date().toISOString(),
      });
    }
  } catch (err) {
    console.error('Message failed:', err);
    messages.value.push({
      id: Date.now() + 1,
      gameSessionId: activeSessionId.value,
      role: 'agent',
      contents: 'Something went wrong. Please try again.',
      createdAt: new Date().toISOString(),
    });
  } finally {
    streaming.value = false;
    streamText.value = '';
    await nextTick();
    scrollToBottom();
  }
}

function scrollToBottom() {
  if (chatArea.value) {
    chatArea.value.scrollTop = chatArea.value.scrollHeight;
  }
}
</script>

<style scoped>
/* --- Loading --- */

.vignette-loading {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}

/* --- Page layout --- */

.vignette-page {
  block-size: 100%;
  display: flex;
  flex-direction: column;
}

.vignette-compose {
  inline-size: var(--size-xl);
  block-size: 100%;
  margin-inline: auto;
  padding: var(--size-8);
  display: flex;
  flex-direction: column;
  gap: var(--size-5);
  overflow-y: auto;
}

.vignette-title-input {
  font-size: var(--font-size-6);
  font-weight: var(--font-weight-7);
  background: none;
  border: var(--border-size-1) solid transparent;
  border-radius: var(--radius-2);
  padding: var(--size-1) var(--size-2);
  color: var(--text-1);
  inline-size: 100%;
  transition: border-color var(--animation-duration, 0.15s) var(--ease-2),
    background var(--animation-duration, 0.15s) var(--ease-2);
}

.vignette-title-input:hover {
  border-color: var(--surface-4);
}

.vignette-title-input:focus {
  outline: none;
  border-color: var(--indigo-6);
  background: var(--surface-2);
}

.vignette-title-input::placeholder {
  color: var(--text-2);
  opacity: 0.5;
}

.vignette-hint {
  font-size: var(--font-size-2);
  color: var(--text-2);
  line-height: var(--font-lineheight-4);
}

.disposition-area {
  inline-size: 100%;
  padding: var(--size-4);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-3);
  background: var(--surface-1);
  color: var(--text-1);
  font-size: var(--font-size-2);
  line-height: var(--font-lineheight-4);
  resize: vertical;
  font-family: inherit;
  min-block-size: var(--size-12);
  transition: border-color var(--animation-duration, 0.15s) var(--ease-2);
}

.disposition-area:focus {
  outline: none;
  border-color: var(--indigo-6);
  box-shadow: 0 0 0 var(--border-size-2) oklch(from var(--indigo-6) l c h / 0.2);
}

.disposition-area::placeholder {
  color: var(--text-2);
  opacity: 0.6;
}

.compose-actions {
  display: flex;
  gap: var(--size-3);
}

/* --- Buttons --- */

.btn {
  padding: var(--size-3) var(--size-8);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  border: none;
  transition: transform var(--animation-duration, 0.15s) var(--ease-2),
    box-shadow var(--animation-duration, 0.15s) var(--ease-2),
    opacity var(--animation-duration, 0.15s) var(--ease-2);
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn--primary {
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.btn--primary:hover:not(:disabled) {
  transform: translateY(calc(var(--size-1) * -1));
  box-shadow: var(--shadow-3);
}

.btn--secondary {
  background: var(--surface-3);
  color: var(--text-1);
}

.btn--secondary:hover:not(:disabled) {
  background: var(--surface-4);
}

.btn--ghost {
  background: transparent;
  color: var(--text-2);
}

.btn--ghost:hover:not(:disabled) {
  color: var(--text-1);
  background: var(--surface-3);
}

.btn--sm {
  padding: var(--size-2) var(--size-5);
  font-size: var(--font-size-1);
}

/* --- Game mode --- */

.vignette-game {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  block-size: 100%;
}

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

.game-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-6);
}

.game-title--input {
  background: none;
  border: var(--border-size-1) solid transparent;
  border-radius: var(--radius-2);
  padding: var(--size-1) var(--size-2);
  color: var(--text-1);
  transition: border-color var(--animation-duration, 0.15s) var(--ease-2),
    background var(--animation-duration, 0.15s) var(--ease-2);
}

.game-title--input:hover {
  border-color: var(--surface-4);
}

.game-title--input:focus {
  outline: none;
  border-color: var(--indigo-6);
  background: var(--surface-2);
}

.game-title--input::placeholder {
  color: var(--text-2);
  opacity: 0.5;
}

.chat-area {
  flex: 1;
  overflow-y: auto;
  padding: var(--size-6);
  display: flex;
  flex-direction: column;
  gap: var(--size-3);
}

.chat-message {
  max-inline-size: 80%;
  border-radius: var(--radius-3);
  padding: var(--size-3) var(--size-4);
  line-height: var(--font-lineheight-4);
  white-space: pre-wrap;
}

.chat-message--user {
  align-self: flex-end;
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.chat-message--agent {
  align-self: flex-start;
  background: var(--surface-2);
  color: var(--text-1);
}

.chat-message-content {
  font-size: var(--font-size-2);
}

.typing {
  font-style: italic;
  opacity: 0.7;
}

.chat-input {
  display: flex;
  gap: var(--size-2);
  padding: var(--size-4) var(--size-6);
  border-block-start: var(--border-size-1) solid var(--surface-3);
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

/* --- Transitions --- */

.fade-enter-active,
.fade-leave-active {
  transition: opacity var(--animation-duration, 0.2s) var(--ease-2);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
