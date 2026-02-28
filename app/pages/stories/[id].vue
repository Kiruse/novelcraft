<template>
  <div v-if="story" class="story-detail">
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
        <button class="start-button">Start Game</button>
      </div>
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

const route = useRoute();
const { data } = await useFetch(`/api/stories/${route.params.id}`);

const story = computed(() => data.value?.story);
</script>

<style scoped>
.story-detail {
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem;
}

.story-header {
  background: white;
  border-radius: 16px;
  overflow: hidden;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.story-hero {
  width: 100%;
  aspect-ratio: 21/9;
  background: #f5f5f5;
  overflow: hidden;
}

.hero-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.hero-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  font-size: 6rem;
  font-weight: bold;
}

.story-info {
  padding: 2rem;
}

.story-title {
  font-size: 2rem;
  font-weight: 700;
  margin: 0 0 0.5rem 0;
  color: #333;
}

.story-meta {
  margin-bottom: 1rem;
  color: #888;
}

.story-author {
  font-weight: 500;
}

.story-description {
  font-size: 1rem;
  line-height: 1.6;
  color: #555;
  margin: 0 0 2rem 0;
}

.start-button {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border: none;
  padding: 0.875rem 2rem;
  font-size: 1rem;
  font-weight: 600;
  border-radius: 8px;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.start-button:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.start-button:active {
  transform: translateY(0);
}

.loading {
  text-align: center;
  padding: 4rem;
  color: #666;
}
</style>
