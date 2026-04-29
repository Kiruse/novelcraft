<template>
  <div class="discovery-page">
    <!-- Vignette quick start -->
    <section class="vignette-section">
      <div class="vignette-hero">
        <h2 class="section-title">
          <span class="vignette-icon">✦</span>
          Vignette
        </h2>
        <p class="vignette-desc">Jump right into a story. Write a few words, get a suggestion, and play.</p>

        <div class="vignette-input-row">
          <input
            v-model="quickDisposition"
            type="text"
            class="vignette-input"
            placeholder="A few keywords or a short premise..."
            @keyup.enter="createVignette"
          />
          <button
            class="btn btn--primary"
            :disabled="creating"
            @click="createVignette"
          >
            {{ creating ? 'Starting...' : 'Start Vignette' }}
          </button>
        </div>
      </div>
    </section>

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

const stories = computed(() => storiesData.value?.stories ?? []);

const quickDisposition = ref('');
const creating = ref(false);

async function createVignette() {
  creating.value = true;
  try {
    await navigateTo(`/vignettes/new?disposition=${encodeURIComponent(quickDisposition.value)}`);
  } catch (e) {
    console.error('Failed to create vignette', e);
  } finally {
    creating.value = false;
  }
}
</script>

<style scoped>
.discovery-page {
  max-inline-size: var(--size-xl);
  margin-inline: auto;
  padding: var(--size-8);
}

/* --- Vignette quick start --- */

.vignette-section {
  margin-block-end: var(--size-10);
}

.vignette-hero {
  background: var(--surface-1);
  border-radius: var(--radius-4);
  padding: var(--size-8);
  box-shadow: var(--shadow-2);
  border: var(--border-size-1) solid var(--surface-3);
}

.vignette-icon {
  color: var(--indigo-6);
  margin-inline-end: var(--size-2);
}

.vignette-desc {
  font-size: var(--font-size-2);
  color: var(--text-2);
  margin-block-end: var(--size-5);
  line-height: var(--font-lineheight-4);
}

.vignette-input-row {
  display: flex;
  gap: var(--size-3);
  align-items: stretch;
}

.vignette-input {
  flex: 1;
  padding: var(--size-3) var(--size-4);
  border: var(--border-size-1) solid var(--surface-4);
  border-radius: var(--radius-2);
  background: var(--surface-2);
  color: var(--text-1);
  font-size: var(--font-size-2);
  transition: border-color var(--animation-duration, 0.15s) var(--ease-2);
}

.vignette-input:focus {
  outline: none;
  border-color: var(--indigo-6);
  box-shadow: 0 0 0 var(--border-size-2) oklch(from var(--indigo-6) l c h / 0.2);
}

.vignette-input::placeholder {
  color: var(--text-2);
  opacity: 0.6;
}

/* --- Buttons --- */

.btn {
  padding: var(--size-3) var(--size-8);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  border: none;
  white-space: nowrap;
  transition: transform var(--animation-duration, 0.15s) var(--ease-2),
    box-shadow var(--animation-duration, 0.15s) var(--ease-2),
    opacity var(--animation-duration, 0.15s) var(--ease-2);
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn--primary {
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.btn--primary:hover:not(:disabled) {
  transform: translateY(calc(var(--size-1) * -1));
  box-shadow: var(--shadow-3);
}

/* --- Sections --- */

.section-title {
  font-size: var(--font-size-5);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-5);
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
