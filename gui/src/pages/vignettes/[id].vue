<template>
  <div v-if="!loaded" class="vignette-loading">Loading...</div>
  <div v-else class="vignette-page">
    <template v-if="pages.length === 0">
      <div class="vignette-compose">
        <input
          v-model="vignetteMeta.title"
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
          v-model="vignetteMeta.description"
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

        <SuggestionPicker
          v-if="showPicker"
          ref="pickerRef"
          :persona="SYSTEM_SUGGEST_VIGNETTE"
          :prompt="suggestionPrompt"
          :known-tags="['title', 'genre', 'description']"
          :loading="suggesting"
          :suggestions="suggestions"
          :selected-index="selectedSuggestionIdx"
          @select="useSuggestion"
          @update:suggestions="suggestions = $event"
          @done="suggesting = false"
        />
      </div>
    </template>

    <template v-else>
      <Game
        :vignette="vignette"
        :streaming="streaming"
        :stream-text="streamText"
        :thoughts="thoughts"
        :token-usage="tokenUsage"
        :prompt-debug="prompt"
        title-placeholder="Untitled Vignette"
        @prompt="onPrompt"
        @update-title="updateTitle"
        @update-page="onUpdatePage"
      />
    </template>
  </div>
</template>

<script setup lang="ts">
import type { InputMode } from '~/components/Game.vue';
import type { ParsedSuggestion } from '~/utils/suggestionParser';
import SuggestionPicker from '~/components/SuggestionPicker.vue';
import Game from '~/components/Game.vue';
import {
  SYSTEM_VIGNETTE_OPEN,
  SYSTEM_STEER,
  SYSTEM_INSTRUCT,
  SYSTEM_SUGGEST_VIGNETTE,
} from '~/prompts';
import { type UpdateOpts, useVignette } from '~/composables/useVignette';
import { useVignettes } from '~/composables/useVignettes';
import { useGame } from '~/composables/useGame';
import { getErrorDisplay } from '~/utils/msgUtils';
import { useToast } from '~/composables/useToast';

const route = useRoute();
const id = computed(() => route.params.id as string);

const suggesting = ref(false);
const showPicker = ref(false);
const pickerRef = ref<{ generate: () => void } | null>(null);
const suggestions = ref<Partial<ParsedSuggestion>[]>([]);
const selectedSuggestion = ref<ParsedSuggestion | null>(null);
const selectedSuggestionIdx = ref<number | null>(null);

const toast = useToast();

const vignette = useVignette(id);
const { meta: vignetteMeta, pages } = vignette;
const loaded = computed(() => vignette.status.value !== 'loading');

const { refresh: refreshVignettes } = useVignettes();
const { status: gameStatus, streamText, thoughts, tokenUsage, prompt, run } = useGame({
  meta: vignetteMeta,
  pages,
});

const streaming = computed(() => gameStatus.value === 'streaming');
const suggestionPrompt = computed(() => vignetteMeta.value.description?.trim() ?? '');

const canLockIn = computed(() =>
  (vignetteMeta.value.description?.trim().length ?? 0) > 0 || selectedSuggestionIdx.value !== null
);

let programmaticDisposition = false;

function resetState() {
  suggestions.value = [];
  suggesting.value = false;
  selectedSuggestion.value = null;
  selectedSuggestionIdx.value = null;
  showPicker.value = false;
  pickerRef.value = null;
}

async function updateTitle(newTitle: string) {
  vignetteMeta.value.title = newTitle.trim() || 'Untitled Vignette';
  await vignette.save();
  refreshVignettes();
}

function onDispositionInput() {
  // Clear selected disposition suggestion when writing their own
  if (!programmaticDisposition && selectedSuggestionIdx.value !== null) {
    selectedSuggestion.value = null;
    selectedSuggestionIdx.value = null;
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
  if (selectedSuggestionIdx.value === idx) {
    selectedSuggestion.value = null;
    selectedSuggestionIdx.value = null;
  } else {
    selectedSuggestion.value = suggestion;
    selectedSuggestionIdx.value = idx;
    if (suggestion.description) {
      programmaticDisposition = true;
      vignetteMeta.value = {
        ...vignetteMeta.value,
        title: suggestion.title,
        description: suggestion.description,
      };
      nextTick(() => { programmaticDisposition = false; });
    }
  }
}

async function lockIn() {
  try {
    await vignette.save();
    const update = await vignette.push({});
    const { response, toolCalls, state } = await run({
      session: readonly(vignette.getGameplaySession()),
      promptId: 'create',
    });
    await update(response, toolCalls, state);
  } catch (err) {
    toast.error('Failed to create vignette: ' + getErrorDisplay(err));
    console.error('Vignette start failed:', err);
  }
}

async function createPage(prompt: string, pageIndex: number) {
  if (streaming.value) return;
  if (pageIndex < 0 || pageIndex >= pages.value.length) {
    console.warn(`createPage: index out of bounds, ${pageIndex} not in [0,${pages.value.length})`);
    return;
  }

  prompt = prompt.trim();

  try {
    const lastPage = pages.value[pages.value.length - 1];
    const update = prompt
      ? await vignette.push({ prompt })
      : await vignette.fork({ pageIndex });
    const session = readonly(vignette.getGameplaySession());

    const { response, toolCalls, state } = await run({
      session,
      promptId: prompt ? 'prompt' : 'expand',
      prependStreamText: prompt ? undefined : (lastPage.prompt ?? undefined),
    });

    await update(response, toolCalls, state);
  } catch (err) {
    toast.error('Failed to create new page: ' + getErrorDisplay(err));
    console.error('Create page failed:', err);
  }
}

async function recreatePage(
  pageIndex: number,
  instruction: string,
  mode: 'steer' | 'instruct',
) {
  if (streaming.value || pageIndex === -1) return;

  const page = pages.value[pages.value.length - 1];

  let system = page.system;

  if (mode === 'steer') {
    if (!system) {
      system = instruction;
    } else {
      system = `${system}\n${instruction}`;
    }
  }

  try {
    const update = await vignette.fork({
      pageIndex,
      system,
    });
    const { response, toolCalls, state } = await run({
      session: readonly(vignette.getGameplaySession()),
      promptId: mode,
      getMessages: (msgs) => {
        if (pageIndex === 0) {
          msgs.unshift({ author: 'system', content: SYSTEM_VIGNETTE_OPEN });
        }
        msgs.push({
          author: 'ai',
          content: page.response!,
        });
        switch (mode) {
          case 'steer': {
            msgs.push({
              author: 'system',
              content: SYSTEM_STEER,
            });
            break;
          }
          case 'instruct': {
            msgs.push(
              {
                author: 'system',
                content: SYSTEM_INSTRUCT,
              },
              {
                author: 'user',
                content: instruction || '(no particular instructions)',
              },
            );
            break;
          }
          default: console.error('Unknown recreate mode:', mode);
        }
        return msgs;
      },
    });
    await update(response, toolCalls, state);
  } catch (err) {
    toast.error('Failed to recreate page: ' + getErrorDisplay(err));
    console.error('Recreate Page failed:', err);
  }
}

function onPrompt(payload: { text: string; mode: InputMode; pageIndex: number }) {
  switch (payload.mode) {
    case 'steer':
      return recreatePage(payload.pageIndex, payload.text, 'steer');
    case 'instruct':
      return recreatePage(payload.pageIndex, payload.text, 'instruct');
    case 'write':
    default:
      return createPage(payload.text, payload.pageIndex);
  }
}

async function onUpdatePage(payload: UpdateOpts) {
  const page = pages.value[payload.pageIndex];
  if (!page) return;
  await vignette.update({
    pageIndex: payload.pageIndex,
    system: payload.system !== undefined ? payload.system : (page.system || null),
    prompt: payload.prompt || page.prompt || null,
    response: payload.response || page.response || null,
  });
}

onMounted(resetState);
watch(id, resetState);
</script>

<style scoped>
.vignette-loading {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}

.vignette-page {
  block-size: 100%;
  display: flex;
  flex-direction: column;
}

.vignette-compose {
  flex: 1;
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
