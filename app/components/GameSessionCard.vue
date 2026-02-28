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
  width: 200px;
  background: white;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
  scroll-snap-align: start;
}

.session-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.session-cover {
  aspect-ratio: 16/9;
  width: 100%;
  background: #f5f5f5;
  overflow: hidden;
}

.cover-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.cover-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  font-size: 2rem;
  font-weight: bold;
}

.session-content {
  padding: 0.75rem;
}

.session-title {
  font-size: 0.95rem;
  font-weight: 600;
  margin: 0 0 0.25rem 0;
  color: #333;
}

.session-date {
  font-size: 0.75rem;
  color: #888;
}
</style>
