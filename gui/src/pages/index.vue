<template>
  <div class="home-page">
    <section class="home-hero">
      <h1 class="home-title">NovelCraft</h1>
      <p class="home-subtitle">Jump right into a story. Write a few words, get a suggestion, and play.</p>
      <RouterLink to="/vignettes/new" class="btn btn--primary">+ New vignette</RouterLink>
    </section>

    <section v-if="recentVignettes.length > 0" class="recent-section">
      <h2 class="section-title">Recent vignettes</h2>
      <div class="recent-list">
        <RouterLink
          v-for="v in recentVignettes"
          :key="v.id"
          :to="`/vignettes/${v.id}`"
          class="recent-row"
        >
          <span class="recent-dot" />
          <span class="recent-title">{{ v.title }}</span>
        </RouterLink>
      </div>
      <RouterLink to="/vignettes" class="view-all-link">View all vignettes →</RouterLink>
    </section>

    <section v-else class="empty-state">
      <p>No vignettes yet. Create one to get started!</p>
    </section>
  </div>
</template>

<script setup lang="ts">
import { useVignetteList } from '~/composables/useVignetteList';

const { vignettes: recentVignettes, refresh } = useVignetteList();

onMounted(async () => {
  try {
    await refresh(4);
  } catch {
    // local DB not available
  }
});
</script>

<style scoped>
.home-page {
  max-inline-size: var(--size-lg);
  margin-inline: auto;
  padding: var(--size-8);
}

.home-hero {
  margin-block-end: var(--size-10);
}

.home-title {
  font-size: var(--font-size-7);
  font-weight: var(--font-weight-8);
  letter-spacing: -0.02em;
  margin-block-end: var(--size-2);
}

.home-subtitle {
  font-size: var(--font-size-3);
  color: var(--text-2);
  line-height: var(--font-lineheight-4);
  margin-block-end: var(--size-6);
  max-inline-size: 40ch;
}

.btn {
  display: inline-block;
  padding: var(--size-3) var(--size-8);
  border-radius: var(--radius-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-6);
  cursor: pointer;
  border: none;
  text-decoration: none;
  transition: transform var(--animation-duration, 0.15s) var(--ease-2),
    box-shadow var(--animation-duration, 0.15s) var(--ease-2);
}

.btn--primary {
  background: var(--brand-gradient);
  color: var(--gray-0);
}

.btn--primary:hover {
  transform: translateY(calc(var(--size-1) * -1));
  box-shadow: var(--shadow-3);
}

.section-title {
  font-size: var(--font-size-4);
  font-weight: var(--font-weight-6);
  margin-block-end: var(--size-4);
}

.recent-list {
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.recent-row {
  display: flex;
  align-items: center;
  gap: var(--size-3);
  padding: var(--size-3) var(--size-4);
  background: var(--surface-1);
  border-radius: var(--radius-3);
  box-shadow: var(--shadow-1);
  text-decoration: none;
  color: inherit;
  transition: background var(--animation-duration, 0.15s) var(--ease-2),
    transform var(--animation-duration, 0.15s) var(--ease-2);
}

.recent-row:hover {
  background: var(--surface-3);
  transform: translateY(calc(var(--size-1) * -1));
}

.recent-dot {
  flex-shrink: 0;
  inline-size: var(--size-2);
  block-size: var(--size-2);
  border-radius: var(--radius-round);
  background: var(--indigo-5);
}

.recent-title {
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-5);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.view-all-link {
  display: inline-block;
  margin-block-start: var(--size-5);
  font-size: var(--font-size-2);
  color: var(--indigo-6);
  text-decoration: none;
  font-weight: var(--font-weight-5);
}

.view-all-link:hover {
  text-decoration: underline;
}

.empty-state {
  text-align: center;
  padding: var(--size-10);
  color: var(--text-2);
}
</style>
