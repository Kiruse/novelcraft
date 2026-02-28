<template>
  <div class="discovery-page">
    <div v-if="sessions.length > 0" class="jump-back-in-section">
      <h2 class="section-title">Jump back in</h2>
      <div class="sessions-scroll">
        <GameSessionCard
          v-for="session in sessions"
          :key="session.id"
          :session="session"
        />
      </div>
    </div>

    <h2 class="section-title">Discover stories</h2>
    <div v-if="stories.length > 0" class="stories-grid">
      <StoryCard
        v-for="story in stories"
        :key="story.id"
        :story="story"
      />
    </div>
    <div v-else class="empty-state">
      <p>No stories available</p>
    </div>
  </div>
</template>

<script setup lang="ts">
const { data: storiesData } = await useFetch('/api/stories');
const { data: sessionsData } = await useFetch('/api/sessions');

const stories = computed(() => storiesData.value?.stories ?? []);
const sessions = computed(() => sessionsData.value?.sessions ?? []);
</script>

<style scoped>
.discovery-page {
  max-width: 1400px;
  margin: 0 auto;
  padding: 2rem;
}

.jump-back-in-section {
  margin-bottom: 3rem;
}

.section-title {
  font-size: 1.5rem;
  font-weight: 600;
  margin-bottom: 1.5rem;
}

.sessions-scroll {
  display: flex;
  gap: 1rem;
  overflow-x: auto;
  padding-bottom: 0.5rem;
  scroll-snap-type: x mandatory;
}

.sessions-scroll::-webkit-scrollbar {
  height: 8px;
}

.sessions-scroll::-webkit-scrollbar-track {
  background: #f1f1f1;
  border-radius: 4px;
}

.sessions-scroll::-webkit-scrollbar-thumb {
  background: #888;
  border-radius: 4px;
}

.stories-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1.5rem;
}

.empty-state {
  text-align: center;
  padding: 4rem;
  color: #666;
}
</style>
