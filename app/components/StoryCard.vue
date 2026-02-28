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
  background: white;
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  transition: transform 0.2s, box-shadow 0.2s;
  text-decoration: none;
  color: inherit;
}

.story-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
}

.story-cover {
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
  font-size: 3rem;
  font-weight: bold;
}

.story-content {
  padding: 1rem;
}

.story-title {
  font-size: 1.1rem;
  font-weight: 600;
  margin: 0 0 0.5rem 0;
  color: #333;
}

.story-description {
  font-size: 0.9rem;
  color: #666;
  margin: 0 0 0.75rem 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.story-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.8rem;
  color: #888;
}

.story-author {
  font-weight: 500;
}
</style>
