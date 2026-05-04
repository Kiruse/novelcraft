<template>
  <div class="session-card">
    <div class="session-cover">
      <img
        v-if="session.story.coverArt"
        :src="session.story.coverArt"
        :alt="session.story.title"
        class="cover-image"
      />
      <div v-else class="cover-placeholder">
        <span>{{ session.story.title.charAt(0).toUpperCase() }}</span>
      </div>
    </div>
    <div class="session-content">
      <h4 class="session-title">{{ session.story.title }}</h4>
      <span class="session-date">{{ formatDate(session.updatedAt) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
interface Story {
  id: number;
  title: string;
  coverArt: string | null;
  genre: string | null;
}

interface GameSession {
  id: number;
  storyId: number;
  story: Story;
  updatedAt: Date | string;
}

const props = defineProps<{
  session: GameSession;
}>();

function formatDate(date: Date | string): string {
  const d = new Date(date);
  const now = new Date();
  const diff = now.getTime() - d.getTime();
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days} days ago`;
  return d.toLocaleDateString();
}
</script>

<style scoped>
.session-card {
  flex-shrink: 0;
  inline-size: var(--size-xl);
  background: var(--surface-1);
  border-radius: var(--radius-3);
  overflow: hidden;
  box-shadow: var(--shadow-2);
  cursor: pointer;
  transition: transform var(--animation-duration, 0.2s) var(--ease-2), box-shadow var(--animation-duration, 0.2s) var(--ease-2);
  scroll-snap-align: start;
}

.session-card:hover {
  transform: translateY(calc(var(--size-1) * -1));
  box-shadow: var(--shadow-3);
}

.session-cover {
  aspect-ratio: 16/9;
  inline-size: 100%;
  background: var(--surface-2);
  overflow: hidden;
}

.cover-image {
  inline-size: 100%;
  block-size: 100%;
  object-fit: cover;
}

.cover-placeholder {
  inline-size: 100%;
  block-size: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--brand-gradient);
  color: var(--gray-0);
  font-size: var(--font-size-5);
  font-weight: var(--font-weight-9);
}

.session-content {
  padding: var(--size-3);
}

.session-title {
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-1);
}

.session-date {
  font-size: var(--font-size-0);
  color: var(--text-2);
}
</style>
