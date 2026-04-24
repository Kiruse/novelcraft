<template>
  <div class="chat-area" ref="scrollContainer">
    <template v-for="msg in messages" :key="msg.id">
      <!-- Narrator: plain prose -->
      <div v-if="!msg.author" class="narrative-block">
        {{ msg.contents }}
      </div>

      <!-- Player's own character -->
      <div v-else-if="msg.author === playerName" class="speech-bubble speech-bubble--player">
        <span class="speech-author">{{ msg.author }}</span>
        <span class="speech-text">{{ msg.contents }}</span>
      </div>

      <!-- Other characters / NPCs -->
      <div v-else class="speech-bubble speech-bubble--character">
        <span class="speech-author">{{ msg.author }}</span>
        <span class="speech-text">{{ msg.contents }}</span>
      </div>
    </template>

    <!-- Live streaming bubble -->
    <div v-if="streaming" class="narrative-block narrative-block--streaming">
      <span v-if="streamText">{{ streamText }}</span>
      <span v-else class="typing">Thinking...</span>
    </div>
  </div>
</template>

<script setup lang="ts">
/** A single piece of content in the chat area. */
export interface ChatMessage {
  id: number | string;
  contents: string;
  /**
   * Who this text is written *as*.
   * `null` / `undefined` = narrator (plain prose, no bubble).
   * Any string = in-character speech, rendered as a bubble.
   */
  author?: string | null;
}

const props = withDefaults(defineProps<{
  messages: ChatMessage[];
  /** The player's character name. Bubbles with this author are right-aligned. */
  playerName?: string;
  streaming?: boolean;
  streamText?: string;
}>(), {
  playerName: undefined,
  streaming: false,
  streamText: '',
});

const scrollContainer = ref<HTMLElement | null>(null);

/** How close to the bottom (in px) counts as "at bottom". */
const SCROLL_THRESHOLD = 80;

function isNearBottom(): boolean {
  const el = scrollContainer.value;
  if (!el) return true;
  return el.scrollHeight - el.scrollTop - el.clientHeight < SCROLL_THRESHOLD;
}

function scrollToBottom() {
  const el = scrollContainer.value;
  if (el) el.scrollTop = el.scrollHeight;
}

// Only auto-scroll if the user is already near the bottom
watch(() => props.messages.length, () => {
  if (isNearBottom()) nextTick(scrollToBottom);
});

watch(() => props.streamText, () => {
  if (isNearBottom()) nextTick(scrollToBottom);
});

watch(() => props.streaming, (v) => {
  if (v && isNearBottom()) nextTick(scrollToBottom);
});

defineExpose({ scrollToBottom, isNearBottom });
</script>

<style scoped>
.chat-area {
  flex: 1;
  overflow-y: auto;
  padding: var(--size-6);
  display: flex;
  flex-direction: column;
  gap: var(--size-3);
}

/* --- Narrator prose --- */

.narrative-block {
  max-inline-size: 100%;
  font-size: var(--font-size-2);
  line-height: var(--font-lineheight-5);
  color: var(--text-1);
  white-space: pre-wrap;
  word-break: break-word;
}

.narrative-block--streaming {
  color: var(--text-2);
}

/* --- Speech bubbles --- */

.speech-bubble {
  max-inline-size: 80%;
  border-radius: var(--radius-3);
  padding: var(--size-3) var(--size-4);
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
}

.speech-bubble--player {
  align-self: flex-end;
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.speech-bubble--character {
  align-self: flex-start;
  background: var(--surface-2);
  color: var(--text-1);
}

.speech-author {
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-6);
  text-transform: uppercase;
  letter-spacing: var(--font-letterspacing-1);
  opacity: 0.7;
}

.speech-text {
  font-size: var(--font-size-2);
  line-height: var(--font-lineheight-4);
  white-space: pre-wrap;
  word-break: break-word;
}

.typing {
  font-style: italic;
  opacity: 0.7;
}
</style>
