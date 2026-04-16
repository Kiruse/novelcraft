<template>
  <NuxtLink :to="`/stories/${story.id}`" class="story-card">
    <div class="story-cover">
      <img
        v-if="story.coverArt"
        :src="story.coverArt"
        :alt="story.title"
        class="cover-image"
      />
      <div v-else class="cover-placeholder">
        <span>{{ story.title.charAt(0).toUpperCase() }}</span>
      </div>
    </div>
    <div class="story-content">
      <h3 class="story-title">{{ story.title }}</h3>
      <p v-if="story.description" class="story-description">
        {{ story.description }}
      </p>
      <div class="story-meta">
        <span class="story-author">{{ story.author.name }}</span>
      </div>
    </div>
  </NuxtLink>
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
  };
}

defineProps<{
  story: Story;
}>();
</script>

<style scoped>
.story-card {
  display: block;
  background: var(--surface-1);
  border-radius: var(--radius-3);
  overflow: hidden;
  box-shadow: var(--shadow-2);
  transition: transform var(--animation-duration, 0.2s) var(--ease-2), box-shadow var(--animation-duration, 0.2s) var(--ease-2);
  text-decoration: none;
  color: inherit;
}

.story-card:hover {
  transform: translateY(calc(var(--size-1) * -1));
  box-shadow: var(--shadow-4);
}

.story-cover {
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
  font-size: var(--font-size-7);
  font-weight: var(--font-weight-9);
}

.story-content {
  padding: var(--size-4);
}

.story-title {
  font-size: var(--font-size-3);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-2);
}

.story-description {
  font-size: var(--font-size-2);
  color: var(--text-2);
  margin-block-end: var(--size-3);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.story-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: var(--font-size-1);
  color: var(--text-2);
}

.story-author {
  font-weight: var(--font-weight-5);
}
</style>
