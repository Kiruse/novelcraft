<template>
  <div v-if="!vignette" class="vignette-loading">Loading...</div>
  <div v-else class="vignette-page">
    <!-- Draft / Compose mode (no game session yet) -->
    <template v-if="!activeSessionId">
      <div class="vignette-compose">
        <input
          v-model="title"
          class="vignette-title-input"
          placeholder="Untitled Vignette"
          @blur="updateTitle(($event.target as HTMLInputElement).value)"
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
          :persona="VIGNETTE_SUGGEST_PERSONA"
          :prompt="suggestionPrompt"
          :known-tags="['title', 'genre', 'description']"
          :loading="suggesting"
          :suggestions="suggestions"
          :completed="completedSet"
          :selected-index="selectedIndex"
          @select="useSuggestion"
          @update:suggestions="suggestions = $event"
          @update:completed="completedSet = $event"
          @done="suggesting = false"
        />
      </div>
    </template>

    <!-- Playing mode (session active) -->
    <template v-else>
      <Game
        :pages="pages"
        :title="title"
        title-placeholder="Untitled Vignette"
        :streaming="streaming"
        :stream-text="streamText"
        @prompt="onPrompt"
        @update-title="updateTitle"
        @update-page="onUpdatePage"
      />
    </template>
  </div>
</template>

<script setup lang="ts">
import type { VignetteShape } from '~/composables/useCurrentUser';
import type { GamePage } from '~/utils/msgUtils';
import type { InputMode } from '~/components/Game.vue';
import type { ParsedSuggestion } from '~/utils/suggestionParser';
import { unindent } from '@stegakir/aikit/utils';
import {
  PERSONA_PLATFORM,
  SYSTEM_VIGNETTE_OPEN,
  SYSTEM_STEER,
  SYSTEM_INSTRUCT,
} from '#shared/prompts';
import { buildMessages, streamLlm } from '~/utils/llmHelpers';

interface VignettePageShape {
  id: number;
  gameSessionId: number;
  system: string | null;
  prompt: string | null;
  response: string | null;
  createdAt: string;
}

// IMPORTANT: DO NOT TOUCH THIS PAGE
// Human is currently completely refactoring this page

const route = useRoute();
const id = computed(() => route.params.id as string);

const disposition = ref('');
const suggesting = ref(false);
const showPicker = ref(false);
const pickerRef = ref<{ generate: () => void } | null>(null);
const suggestions = ref<Partial<ParsedSuggestion>[]>([]);
const completedSet = ref<Set<number>>(new Set());
const selectedSuggestion = ref<ParsedSuggestion | null>(null);
const selectedIndex = ref<number | null>(null);
const streaming = ref(false);
const streamText = ref('');

const VIGNETTE_SUGGEST_PERSONA = unindent(`
  You are a creative story premise generator for quick interactive vignettes.
  The user will provide a disposition — either a few keywords, a theme, or a full paragraph
  describing the kind of story they want to experience.

  Generate exactly 3 diverse story suggestions. Each must have a unique genre and tone.
  For each suggestion, output a <suggestion> block with these tags inside:
    <title> — a short, catchy title (3-6 words)</title>
    <genre> — the genre</genre>
    <description> — a vivid 2-3 sentence premise that sets the scene and ends with a hook.
    Write descriptions in second person ("You..."). Make them evocative and specific.</description>

  Output ONLY the <suggestion> blocks with no other text.
`);

const { refresh: refreshUserData } = useCurrentUser();

const { data, refresh: refreshData } = await useFetch<{
  vignette: VignetteShape;
  session: { id: number } | null;
  pages: VignettePageShape[];
}>(`/api/vignettes/${id.value}`);

const vignette = computed(() => data.value?.vignette);
const title = ref(data.value?.vignette?.title ?? 'Untitled Vignette');
const activeSessionId = computed(() => data.value?.session?.id ?? null);
const pages = ref<GamePage[]>(
  (data.value?.pages ?? []).map(p => ({
    id: p.id,
    system: p.system,
    prompt: p.prompt,
    response: p.response,
  })),
);

const suggestionPrompt = computed(() =>
  disposition.value.trim()
    ? `Here is my disposition:\n\n${disposition.value.trim()}\n\nGenerate 3 story suggestions based on this.`
    : 'Generate 3 random creative story suggestions for interactive vignettes. Surprise me with variety.',
);

// Whether the "Lock In" (disposition) button is enabled.
const canLockIn = computed(() => {
  return disposition.value.trim().length > 0 || selectedIndex.value !== null;
});

// Clear suggestion selection when user edits the textarea
let programmaticDisposition = false;

async function updateTitle(newTitle: string) {
  newTitle = newTitle.trim() || 'Untitled Vignette';
  title.value = newTitle;

  try {
    await $fetch(`/api/vignettes/${id.value}`, {
      method: 'PATCH',
      body: { title: newTitle },
    });
    await refreshUserData();
  } catch (e) {
    console.error('Failed to save title', e);
  }
}

function onDispositionInput() {
  if (!programmaticDisposition && selectedIndex.value !== null) {
    selectedSuggestion.value = null;
    selectedIndex.value = null;
  }
}

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
  const idx = suggestions.value.findIndex(
    s => s.title === suggestion.title && s.description === suggestion.description,
  );
  if (selectedIndex.value === idx) {
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

  streaming.value = true;
  streamText.value = '';

  // Optimistically add an empty page for the opening
  const tempId = `temp-${Date.now()}`;
  pages.value.push({ id: tempId, system: null, prompt: null, response: null });

  try {
    // 1. Create session + page on server (no sessionId = creates new session)
    const { sessionId, pageId } = await $fetch<{ sessionId: number; pageId: number }>(
      `/api/vignettes/${id.value}/message`,
      { method: 'POST', body: JSON.stringify({}) },
    );

    await refreshData();

    // 2. Generate opening via LLM
    const context = vignette.value?.description
      ? `Title: ${title.value}\n\nPremise:\n${vignette.value.description}`
      : `Setup:\n${vignette.value?.description ?? ''}`;

    const messages = [
      { author: 'system', content: SYSTEM_VIGNETTE_OPEN },
      { author: 'user', content: `Start the vignette based on this setup:\n\n${context}` },
    ];

    let fullText = '';
    const streamGen = streamLlm({
      persona: PERSONA_PLATFORM,
      messages,
    });
    for await (const chunk of streamGen) {
      fullText += chunk;
      streamText.value += chunk;
    }

    // 3. Update local state
    const idx = pages.value.findIndex(p => p.id === tempId);
    if (idx !== -1) {
      pages.value[idx] = {
        id: pageId,
        system: null,
        prompt: null,
        response: fullText,
      };
    }

    // 4. Persist response
    await $fetch(`/api/vignettes/${id.value}/pages/${pageId}`, {
      method: 'PATCH',
      body: { response: fullText },
    });
  } catch (err) {
    console.error('Start failed:', err);
    const idx = pages.value.findIndex(p => p.id === tempId);
    if (idx !== -1) pages.value.splice(idx, 1);
  } finally {
    streaming.value = false;
    streamText.value = '';
  }
}

/** Dispatch prompt based on input mode. */
function onPrompt(payload: { text: string; mode: InputMode; pageId: number | string | null }) {
  switch (payload.mode) {
    case 'steer':
      return regeneratePage(payload.pageId, payload.text, 'steer');
    case 'instruct':
      return regeneratePage(payload.pageId, payload.text, 'instruct');
    case 'write':
    default:
      return sendWrite(payload.text);
  }
}

/** /write — create a new page, generate via LLM, save. */
async function sendWrite(text: string) {
  if (streaming.value || !activeSessionId.value) return;

  const isWriteMore = !text;
  const systemNote = isWriteMore ? 'Continue the story — write more.' : null;

  // Optimistically add a new page
  const tempId = `temp-${Date.now()}`;
  pages.value.push({
    id: tempId,
    system: systemNote,
    prompt: text || null,
    response: null,
  });

  streaming.value = true;
  streamText.value = '';

  try {
    // 1. Create page on server
    const { pageId } = await $fetch<{ pageId: number }>(
      `/api/vignettes/${id.value}/message`,
      {
        method: 'POST',
        body: JSON.stringify({
          content: text || null,
          system: systemNote,
          sessionId: activeSessionId.value,
        }),
      },
    );

    // 2. Generate response via LLM
    const messages = buildMessages({
      title: title.value,
      description: vignette.value?.description,
      pages: pages.value,
      lastPageOverride: { response: null },
    });
    let fullText = '';
    const streamGen = streamLlm({
      persona: PERSONA_PLATFORM,
      messages,
    });
    for await (const chunk of streamGen) {
      fullText += chunk;
      streamText.value += chunk;
    }

    // 3. Update local state
    const idx = pages.value.findIndex(p => p.id === tempId);
    if (idx !== -1) {
      pages.value[idx] = {
        id: pageId,
        system: systemNote,
        prompt: text || null,
        response: fullText,
      };
    }

    // 4. Persist response
    await $fetch(`/api/vignettes/${id.value}/pages/${pageId}`, {
      method: 'PATCH',
      body: { response: fullText },
    });
  } catch (err) {
    console.error('Message failed:', err);
    const idx = pages.value.findIndex(p => p.id === tempId);
    if (idx !== -1) {
      pages.value[idx] = {
        ...pages.value[idx]!,
        response: 'Something went wrong. Please try again.',
      };
    }
  } finally {
    streaming.value = false;
    streamText.value = '';
  }
}

/** /steer or /instruct — regenerate the current page via LLM. */
async function regeneratePage(
  pageId: number | string | null,
  instruction: string,
  mode: 'steer' | 'instruct',
) {
  if (!instruction || streaming.value || !activeSessionId.value || !pageId) return;

  const pageIdx = pages.value.findIndex(p => p.id === pageId);
  if (pageIdx === -1) return;

  const oldPage = pages.value[pageIdx]!;

  // For steer, optimistically append the instruction to the system field
  if (mode === 'steer') {
    const updatedSystem = oldPage.system
      ? `${oldPage.system}\n${instruction}`
      : instruction;
    pages.value[pageIdx] = { ...oldPage, system: updatedSystem };
  }

  streaming.value = true;
  streamText.value = '';

  try {
    const currentPage = pages.value[pageIdx]!;
    const systemPage = currentPage.system;

    // For steer, persist the updated system
    if (mode === 'steer' && typeof pageId === 'number') {
      await $fetch(`/api/vignettes/${id.value}/pages/${pageId}`, {
        method: 'PATCH',
        body: { system: systemPage },
      });
    }

    // Build editor instruction message
    const modeReminder = mode === 'steer' ? SYSTEM_STEER : SYSTEM_INSTRUCT;

    const systemParts = [modeReminder];
    if (systemPage) systemParts.push(systemPage);

    const messages = buildMessages({
      title: title.value,
      description: vignette.value?.description,
      pages: pages.value,
      lastPageOverride: { response: null },
    });

    messages.push({
      author: 'system',
      content: systemParts.join('\n\n'),
    });

    let fullText = '';
    const streamGen = streamLlm({
      persona: PERSONA_PLATFORM,
      messages,
    });
    for await (const chunk of streamGen) {
      fullText += chunk;
      streamText.value += chunk;
    }

    // Update the page with the regenerated response
    const currentIdx = pages.value.findIndex(p => p.id === pageId);
    if (currentIdx !== -1) {
      pages.value[currentIdx] = {
        ...pages.value[currentIdx]!,
        response: fullText || pages.value[currentIdx]!.response,
      };
    }

    // Persist response
    if (typeof pageId === 'number') {
      await $fetch(`/api/vignettes/${id.value}/pages/${pageId}`, {
        method: 'PATCH',
        body: { response: fullText },
      });
    }
  } catch (err) {
    console.error('Regenerate failed:', err);
    if (mode === 'steer') {
      pages.value[pageIdx] = oldPage;
    }
  } finally {
    streaming.value = false;
    streamText.value = '';
  }
}

/** Handle page edits from the Game component. */
async function onUpdatePage(payload: { pageId: number | string; response?: string; system?: string | null }) {
  const idx = pages.value.findIndex(p => p.id === payload.pageId);
  if (idx !== -1) {
    const current = pages.value[idx]!;
    pages.value[idx] = {
      ...current,
      ...(payload.response !== undefined ? { response: payload.response } : {}),
      ...(payload.system !== undefined ? { system: payload.system } : {}),
    };
  }

  if (typeof payload.pageId === 'number') {
    const body: Record<string, string | null> = {};
    if (payload.response !== undefined) body.response = payload.response;
    if (payload.system !== undefined) body.system = payload.system;

    try {
      await $fetch(`/api/vignettes/${id.value}/pages/${payload.pageId}`, {
        method: 'PATCH',
        body,
      });
    } catch (e) {
      console.error('Failed to save page edit', e);
    }
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
</style>
