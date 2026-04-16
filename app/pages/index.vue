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
  max-inline-size: var(--size-xl);
  margin-inline: auto;
  padding: var(--size-8);
}

.jump-back-in-section {
  margin-block-end: var(--size-10);
}

.section-title {
  font-size: var(--font-size-5);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-5);
}

.sessions-scroll {
  display: flex;
  gap: var(--size-4);
  overflow-x: auto;
  padding-block-end: var(--size-2);
  scroll-snap-type: x mandatory;
}

.sessions-scroll::-webkit-scrollbar {
  block-size: var(--size-2);
}

.sessions-scroll::-webkit-scrollbar-track {
  background: var(--gray-2);
  border-radius: var(--radius-round);
}

.sessions-scroll::-webkit-scrollbar-thumb {
  background: var(--gray-6);
  border-radius: var(--radius-round);
}

.stories-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(var(--size-content-1), 1fr));
  gap: var(--size-5);
}

.empty-state {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}
</style>
