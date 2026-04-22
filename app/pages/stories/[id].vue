<template>
  <div v-if="story" class="story-detail">
    <!-- Story detail view -->
    <div v-if="!gameSession" class="story-header">
      <div class="story-hero">
        <img
          v-if="story.coverArt"
          :src="story.coverArt"
          :alt="story.title"
          class="hero-image"
        />
        <div v-else class="hero-placeholder">
          <span>{{ story.title.charAt(0).toUpperCase() }}</span>
        </div>
      </div>
      <div class="story-info">
        <h1 class="story-title">{{ story.title }}</h1>
        <div class="story-meta">
          <span class="story-author">By {{ story.author.name }}</span>
        </div>
        <p v-if="story.description" class="story-description">
          {{ story.description }}
        </p>
        <button class="start-button" @click="startGame" :disabled="starting">
          {{ starting ? 'Starting...' : 'Start Game' }}
        </button>
      </div>
    </div>

    <!-- Game view -->
    <div v-else class="game-view">
      <div class="game-header">
        <h1 class="game-title">{{ story.title }}</h1>
        <button v-if="isTestMode" type="button" class="debug-toggle" @click="debugOpen = !debugOpen">
          {{ debugOpen ? '✕ Close Debug' : '🐛 Debug' }}
        </button>
      </div>
      <div class="chat-area" ref="chatArea">
        <div
          v-for="(msg, i) in messages"
          :key="i"
          :class="['chat-message', `chat-message--${msg.role}`]"
        >
          <div class="chat-message-content">{{ msg.content }}</div>
        </div>
        <div v-if="loading" class="chat-message chat-message--agent">
          <div class="chat-message-content typing">Thinking...</div>
        </div>
      </div>
      <form class="chat-input" @submit.prevent="sendMessage">
        <input
          v-model="input"
          type="text"
          placeholder="What do you do?"
          :disabled="loading"
          class="chat-input-field"
          autofocus
        />
        <button type="submit" :disabled="loading || !input.trim()" class="chat-input-send">
          Send
        </button>
      </form>

      <!-- Debug panel -->
      <GameDebugPanel
        v-if="isTestMode"
        v-model:open="debugOpen"
        :session-id="gameSession.id"
      />
    </div>
  </div>
  <div v-else class="loading">
    Loading...
  </div>
</template>

<script setup lang="ts">
interface Story {
  id: number;
  title: string;
  description: string | null;
  coverArt: string | null;
  genre: string | null;
  author: {
    id: string;
    name: string;
    image: string | null;
  };
}

interface ChatMessage {
  role: 'user' | 'agent';
  content: string;
}

const route = useRoute();
const isTestMode = computed(() => route.query.test === '1');
const debugOpen = ref(false);
const { data } = await useFetch<{
  story: {
    id: number;
    title: string;
    description: string | null;
    coverArt: string | null;
    author: { name: string };
  };
}>(`/api/stories/${route.params.id}`);

const story = computed(() => data.value?.story);

const gameSession = ref<{ id: number } | null>(null);
const messages = ref<ChatMessage[]>([]);
const input = ref('');
const loading = ref(false);
const starting = ref(false);
const chatArea = ref<HTMLElement | null>(null);

// Auto-start in test mode
onMounted(() => {
  if (isTestMode.value && story.value) {
    debugOpen.value = true;
    startGame();
  }
});

async function startGame() {
  starting.value = true;
  try {
    const res: any = await $fetch('/api/sessions', {
      method: 'POST',
      body: { storyId: story.value!.id },
    });
    gameSession.value = res.session;
  } catch (e) {
    console.error('Failed to start game', e);
  } finally {
    starting.value = false;
  }
}

async function sendMessage() {
  const text = input.value.trim();
  if (!text || loading.value || !gameSession.value) return;

  messages.value.push({ role: 'user', content: text });
  input.value = '';
  loading.value = true;

  await nextTick();
  scrollToBottom();

  try {
    const res: any = await $fetch(`/api/sessions/${gameSession.value.id}`, {
      method: 'POST',
      body: { content: text },
    });
    messages.value.push({ role: 'agent', content: res.response });
  } catch (e) {
    console.error('Failed to send message', e);
    messages.value.push({ role: 'agent', content: 'Something went wrong. Please try again.' });
  } finally {
    loading.value = false;
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
.story-detail {
  max-inline-size: var(--size-content-3);
  margin-inline: auto;
  padding: var(--size-8);
}

.story-header {
  background: var(--surface-1);
  border-radius: var(--radius-4);
  overflow: hidden;
  box-shadow: var(--shadow-2);
}

.story-hero {
  inline-size: 100%;
  aspect-ratio: 21/9;
  background: var(--surface-2);
  overflow: hidden;
}

.hero-image {
  inline-size: 100%;
  block-size: 100%;
  object-fit: cover;
}

.hero-placeholder {
  inline-size: 100%;
  block-size: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--brand-gradient);
  color: var(--gray-0);
  font-size: var(--font-size-8);
  font-weight: var(--font-weight-9);
}

.story-info {
  padding: var(--size-8);
}

.story-title {
  font-size: var(--font-size-6);
  font-weight: var(--font-weight-7);
  margin-block-end: var(--size-2);
}

.story-meta {
  margin-block-end: var(--size-4);
  color: var(--text-2);
}

.story-author {
  font-weight: var(--font-weight-5);
}

.story-description {
  font-size: var(--font-size-2);
  line-height: var(--font-lineheight-4);
  color: var(--text-2);
  margin-block-end: var(--size-8);
}

.start-button {
  background: var(--brand-gradient);
  color: var(--gray-0);
  border: none;
  padding: var(--size-3) var(--size-8);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  border-radius: var(--radius-2);
  cursor: pointer;
  transition: transform var(--animation-duration, 0.2s) var(--ease-2), box-shadow var(--animation-duration, 0.2s) var(--ease-2);
}

.start-button:hover:not(:disabled) {
  transform: translateY(calc(var(--size-1) * -1));
  box-shadow: var(--shadow-3);
}

.start-button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Game view */
.game-view {
  display: flex;
  flex-direction: column;
  block-size: calc(100dvh - var(--size-8) * 2);
  background: var(--surface-1);
  border-radius: var(--radius-4);
  overflow: hidden;
  box-shadow: var(--shadow-2);
  transition: margin-inline-end var(--animation-duration, 0.2s) var(--ease-2);
}

.game-view:has(+ .debug-open-placeholder) {
  margin-inline-end: var(--size-content-2);
}

.game-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-4) var(--size-6);
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.game-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-6);
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

.debug-toggle {
  padding: var(--size-2) var(--size-4);
  background: var(--surface-3);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-5);
  cursor: pointer;
  color: var(--text-2);
}

.debug-toggle:hover {
  background: var(--surface-4);
  color: var(--text-1);
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
  transition: transform var(--animation-duration, 0.2s) var(--ease-2);
}

.chat-input-send:hover:not(:disabled) {
  transform: translateY(calc(var(--size-px-1) * -1));
}

.chat-input-send:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.loading {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}
</style>
