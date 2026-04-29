<template>
  <div v-if="error" class="not-found">
    <h1>Story not found</h1>
    <p>{{ error.statusMessage }}</p>
    <NuxtLink to="/" class="back-link">Back to home</NuxtLink>
  </div>
  <div v-else-if="story" :class="['story-page', { 'story-page--game': activeSession }]">
    <!-- Game view (session active) -->
    <template v-if="activeSession">
      <div class="game-layout">
        <div class="game-view">
          <div class="game-header">
            <NuxtLink
              :to="`/stories/${author}/${storySlug}`"
              class="game-back"
            >
              ← Back
            </NuxtLink>
            <h1 class="game-title">{{ story.title }}</h1>
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
        </div>

        <!-- Debug panel -->
        <GameDebugPanel
          v-if="isTestMode"
          :session-id="activeSession.id"
        />
      </div>
    </template>

    <!-- Story detail view -->
    <template v-else>
      <div class="story-header">
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
          <div class="story-actions">
            <button class="start-button" @click="startGame" :disabled="starting">
              {{ starting ? 'Starting...' : 'Start New Session' }}
            </button>
            <NuxtLink
              v-if="isAuthor"
              :to="`/stories/${author}/${storySlug}/edit`"
              class="edit-button"
            >
              Edit Story
            </NuxtLink>
          </div>
        </div>
      </div>

      <!-- Existing sessions -->
      <section v-if="existingSessions.length > 0" class="sessions-section">
        <div v-if="regularSessions.length > 0">
          <h2 class="sessions-title">Your sessions</h2>
          <div class="sessions-list">
            <NuxtLink
              v-for="s in regularSessions"
              :key="s.id"
              :to="`/stories/${author}/${storySlug}?session=${s.id}`"
              class="session-row"
            >
              <span class="session-date">{{ formatDate(s.updatedAt) }}</span>
              <span class="session-resume">Resume →</span>
            </NuxtLink>
          </div>
        </div>

        <div v-if="testSessions.length > 0">
          <h2 class="sessions-title">Test Sessions</h2>
          <div class="sessions-list">
            <NuxtLink
              v-for="s in testSessions"
              :key="s.id"
              :to="`/stories/${author}/${storySlug}?session=${s.id}&test=1`"
              class="session-row"
            >
              <span class="session-date">{{ formatDate(s.updatedAt) }}</span>
              <span class="session-resume">Resume →</span>
            </NuxtLink>
          </div>
        </div>
      </section>
    </template>
  </div>
  <div v-else class="loading">
    Loading...
  </div>
</template>

<script setup lang="ts">
interface ChatMessage {
  role: 'user' | 'agent';
  content: string;
}

const route = useRoute();
const author = route.params.author as string;
const storySlug = route.params.id as string;
const isTestMode = computed(() => route.query.test === '1');
const sessionParam = computed(() => route.query.session as string | undefined);

const { currentUser } = useCurrentUser();
const isAuthor = computed(() => {
  if (!currentUser.value || !story.value) return false;
  return currentUser.value.id === story.value.author?.id;
});

const { data, error } = await useFetch<{
  story: {
    id: number;
    title: string;
    description: string | null;
    coverArt: string | null;
    author: { id: string; name: string };
  };
}>(`/api/stories/${author}/${storySlug}`, {
  query: { test: isTestMode.value ? '1' : undefined },
});

const story = computed(() => data.value?.story);

// --- Existing sessions ---
const { data: sessionsData } = await useFetch<{ sessions: Array<{ id: number; updatedAt: string; story: { version: number } }> }>(
  `/api/stories/${author}/${storySlug}/sessions`,
);
const existingSessions = computed(() => sessionsData.value?.sessions ?? []);
const regularSessions = computed(() => existingSessions.value.filter(s => s.story.version > 0));
const testSessions = computed(() => existingSessions.value.filter(s => s.story.version === 0));

// --- Active session ---
const activeSession = ref<{ id: number } | null>(null);
const messages = ref<ChatMessage[]>([]);
const input = ref('');
const loading = ref(false);
const starting = ref(false);
const chatArea = ref<HTMLElement | null>(null);

// Restore session from ?session=<id>
watch(sessionParam, (id) => {
  if (id) {
    activeSession.value = { id: parseInt(id) };
  } else {
    activeSession.value = null;
    messages.value = [];
  }
}, { immediate: true });

async function startGame() {
  starting.value = true;
  try {
    console.warn('Story play mode is temporarily disabled (migrating to local-first architecture)');
  } catch (e) {
    console.error('Failed to start game', e);
  } finally {
    starting.value = false;
  }
}

async function sendMessage() {
  const text = input.value.trim();
  if (!text || loading.value || !activeSession.value) return;

  messages.value.push({ role: 'user', content: text });
  input.value = '';
  loading.value = true;

  await nextTick();
  scrollToBottom();

  try {
    console.warn('Story play mode is temporarily disabled (migrating to local-first architecture)');
    messages.value.push({ role: 'agent', content: 'Story play mode is temporarily disabled while we migrate to local-first architecture.' });
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

function formatDate(date: string): string {
  const d = new Date(date);
  const now = new Date();
  const diff = now.getTime() - d.getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7) return `${days} days ago`;
  return d.toLocaleDateString();
}
</script>

<style scoped>
.story-page {
  max-inline-size: var(--size-lg);
  margin-inline: auto;
  padding: var(--size-8);
}

.story-page--game {
  max-inline-size: none;
  margin-inline: 0;
  padding: 0;
  block-size: 100%;
  display: flex;
}

/* --- Story header --- */

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

.story-actions {
  display: flex;
  gap: var(--size-3);
  align-items: center;
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

.edit-button {
  padding: var(--size-3) var(--size-8);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  background: var(--surface-2);
  color: var(--text-1);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-5);
  text-decoration: none;
  cursor: pointer;
  transition: background var(--animation-duration, 0.15s) var(--ease-2);
}

.edit-button:hover {
  background: var(--surface-3);
}

/* --- Sessions list --- */

.sessions-section {
  margin-block-start: var(--size-8);
}

.sessions-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-4);
}

.sessions-list {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.session-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-3) var(--size-4);
  background: var(--surface-1);
  border-radius: var(--radius-2);
  box-shadow: var(--shadow-1);
  text-decoration: none;
  color: inherit;
  transition: background var(--animation-duration, 0.15s) var(--ease-2);
}

.session-row:hover {
  background: var(--surface-3);
}

.session-date {
  font-size: var(--font-size-2);
  color: var(--text-2);
}

.session-resume {
  font-size: var(--font-size-2);
  color: var(--indigo-6);
  font-weight: var(--font-weight-5);
}

/* --- Game layout --- */

.game-layout {
  display: flex;
  flex: 1;
  block-size: 100%;
}

.game-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
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
  transition: transform var(--animation-duration, 0.2s) var(--ease-2);
}

.chat-input-send:hover:not(:disabled) {
  transform: translateY(calc(var(--size-px-1) * -1));
}

.chat-input-send:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* --- States --- */

.loading {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}

.not-found {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}

.not-found h1 {
  font-size: var(--font-size-5);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-3);
}

.not-found p {
  margin-block-end: var(--size-4);
}

.back-link {
  color: var(--indigo-6);
  text-decoration: none;
}

.back-link:hover {
  text-decoration: underline;
}
</style>
