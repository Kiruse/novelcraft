<template>
  <aside
    class="sidebar"
    :class="{
      'sidebar--collapsed': !expanded,
      'sidebar--drawer': isMobile,
      'sidebar--open': isMobile && drawerOpen,
    }"
  >
    <!-- Mobile backdrop -->
    <div
      v-if="isMobile && drawerOpen"
      class="sidebar-backdrop"
      @click="drawerOpen = false"
    />

    <div class="sidebar-inner">
      <!-- Header -->
      <div class="sidebar-header">
        <RouterLink to="/" class="sidebar-logo" @click="closeDrawer">
          <span class="logo-mark">◆</span>
          <span v-if="expanded || isMobile" class="logo-text">NovelCraft</span>
        </RouterLink>
        <button
          class="toggle-btn"
          :aria-label="expanded ? 'Collapse sidebar' : 'Expand sidebar'"
          @click="toggle"
        >
          <span class="toggle-icon">{{ expanded || (isMobile && drawerOpen) ? '«' : '»' }}</span>
        </button>
      </div>

      <!-- Nav -->
      <nav class="sidebar-nav">
        <RouterLink to="/" class="sidebar-item" @click="closeDrawer">
          <HomeOutlined class="sidebar-item-icon" />
          <span v-if="expanded || isMobile" class="sidebar-item-label">Home</span>
        </RouterLink>
        <RouterLink
          to="/builder"
          class="sidebar-item"
          @click="closeDrawer"
        >
          <BuildOutlined class="sidebar-item-icon" />
          <span v-if="expanded || isMobile" class="sidebar-item-label">Builder</span>
        </RouterLink>
      </nav>

      <!-- Vignettes (always shown, reads from local DB) -->
      <div v-if="expanded || isMobile" class="sidebar-section">
        <h3 class="sidebar-section-title">Vignettes</h3>
        <button class="sidebar-new-btn" style="margin-bottom: var(--size-1)" @click="router.push('/vignettes/new'); closeDrawer()">
          + New vignette
        </button>
        <div v-if="vignettes.length > 0" class="sidebar-sessions">
          <RouterLink
            v-for="v in vignettes"
            :key="v.id"
            :to="`/vignettes/${v.id}`"
            class="sidebar-session"
            @click="closeDrawer"
          >
            <span class="session-dot" />
            <span class="session-title">{{ v.title }}</span>
          </RouterLink>
          <RouterLink to="/vignettes" class="sidebar-more" @click="closeDrawer">
            View all
          </RouterLink>
        </div>
        <p v-else class="sidebar-empty">No vignettes yet</p>
      </div>
    </div>

    <!-- Account box pinned to bottom -->
    <AccountBox
      :expanded="expanded || isMobile"
      @close-drawer="closeDrawer"
    />
  </aside>
</template>

<script setup lang="ts">
import { HomeOutlined, BuildOutlined } from '@ant-design/icons-vue';
import AccountBox from '~/components/AccountBox.vue';
import { useVignettes } from '~/composables/useVignettes';

const router = useRouter();

const { recent: vignettes, hasMore: hasMoreVignettes, refresh: refreshVignettes } = useVignettes();

const MOBILE_BREAKPOINT = 768;

const expanded = ref(true);
const drawerOpen = ref(false);
const isMobile = ref(false);

function toggle() {
  if (isMobile.value) {
    drawerOpen.value = !drawerOpen.value;
  } else {
    expanded.value = !expanded.value;
  }
}

defineExpose({ toggle });

function closeDrawer() {
  if (isMobile.value) drawerOpen.value = false;
}

function checkMobile() {
  isMobile.value = window.innerWidth < MOBILE_BREAKPOINT;
  if (!isMobile.value) drawerOpen.value = false;
}

onMounted(async () => {
  checkMobile();
  window.addEventListener('resize', checkMobile);
  try {
    await refreshVignettes();
  } catch {
    // local DB not available
  }
});

onUnmounted(() => {
  window.removeEventListener('resize', checkMobile);
});
</script>

<style scoped>
.sidebar {
  --sidebar-width: var(--size-13);
  --sidebar-collapsed: 3.5rem;
  position: sticky;
  inset-block-start: 0;
  display: flex;
  flex-direction: column;
  inline-size: var(--sidebar-width);
  block-size: 100dvh;
  background: var(--surface-2);
  border-inline-end: var(--border-size-1) solid var(--surface-3);
  transition: inline-size var(--animation-duration, 0.2s) var(--ease-2);
  flex-shrink: 0;
  overflow: hidden;
}

.sidebar--collapsed {
  inline-size: var(--sidebar-collapsed);
}

/* --- Mobile drawer --- */

@media (max-width: 767px) {
  .sidebar {
    position: fixed;
    inset-block-start: 0;
    inset-inline-start: 0;
    block-size: 100dvh;
    inline-size: var(--sidebar-width);
    transform: translateX(-100%);
    transition: transform var(--animation-duration, 0.2s) var(--ease-2);
    z-index: var(--layer-5, 200);
  }

  .sidebar--open {
    transform: translateX(0);
  }
}

.sidebar-backdrop {
  position: fixed;
  inset: 0;
  background: oklch(from var(--gray-9) l c h / 0.5);
  z-index: -1;
}

.sidebar-inner {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

/* --- Header --- */

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--size-4);
  gap: var(--size-2);
}

.sidebar-logo {
  display: flex;
  align-items: center;
  gap: var(--size-2);
  text-decoration: none;
  color: var(--text-1);
  min-inline-size: 0;
}

.logo-mark {
  font-size: var(--font-size-4);
  flex-shrink: 0;
}

.logo-text {
  font-size: var(--font-size-3);
  font-weight: var(--font-weight-8);
  letter-spacing: -0.02em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.toggle-btn {
  background: none;
  border: none;
  cursor: pointer;
  padding: var(--size-1) var(--size-2);
  border-radius: var(--radius-2);
  color: var(--text-2);
  flex-shrink: 0;
}

.toggle-btn:hover {
  background: var(--surface-3);
  color: var(--text-1);
}

.toggle-icon {
  font-size: var(--font-size-2);
}

/* --- Nav --- */

.sidebar-nav {
  display: flex;
  flex-direction: column;
  padding: var(--size-2);
  gap: var(--size-1);
}

.sidebar-item {
  display: flex;
  align-items: center;
  gap: var(--size-3);
  padding: var(--size-2) var(--size-3);
  border-radius: var(--radius-2);
  text-decoration: none;
  color: var(--text-2);
  font-size: var(--font-size-2);
  font-weight: var(--font-weight-5);
  white-space: nowrap;
}

.sidebar-item:hover {
  background: var(--surface-3);
  color: var(--text-1);
}

.sidebar-item-icon {
  flex-shrink: 0;
  font-size: var(--font-size-3);
  line-height: 1;
}

/* --- Sessions section --- */

.sidebar-section {
  padding: var(--size-2) var(--size-3);
  margin-block-start: var(--size-4);
}

.sidebar-section-title {
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-6);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-2);
  padding: var(--size-1) var(--size-2);
  margin-block-end: var(--size-1);
}

.sidebar-section-subtitle {
  font-size: var(--font-size-0);
  font-weight: var(--font-weight-5);
  letter-spacing: 0.05em;
  color: var(--text-2);
  padding: var(--size-2) var(--size-2) var(--size-1);
  margin-block-start: var(--size-2);
  border-block-start: var(--border-size-1) solid var(--surface-3);
}

.sidebar-sessions {
  display: flex;
  flex-direction: column;
  gap: var(--size-1);
}

.sidebar-session {
  display: flex;
  align-items: center;
  gap: var(--size-2);
  padding: var(--size-2) var(--size-3);
  border-radius: var(--radius-2);
  text-decoration: none;
  color: var(--text-1);
  font-size: var(--font-size-1);
  white-space: nowrap;
  overflow: hidden;
}

.sidebar-session:hover {
  background: var(--surface-3);
}

.session-dot {
  flex-shrink: 0;
  inline-size: var(--size-2);
  block-size: var(--size-2);
  border-radius: var(--radius-round);
  background: var(--indigo-5);
}

.session-dot--test {
  background: var(--amber-5);
}

.session-dot--story {
  background: var(--green-5);
}

.sidebar-more {
  display: block;
  font-size: var(--font-size-0);
  color: var(--indigo-6);
  text-decoration: none;
  padding: var(--size-2) var(--size-3);
  font-weight: var(--font-weight-5);
}

.sidebar-more:hover {
  text-decoration: underline;
}

.sidebar-new-btn {
  display: block;
  margin-block-start: var(--size-2);
  inline-size: 100%;
  padding: var(--size-2) var(--size-3);
  border-radius: var(--radius-2);
  background: none;
  color: var(--text-2);
  border: var(--border-size-1) solid transparent;
  font-size: var(--font-size-1);
  font-weight: var(--font-weight-5);
  cursor: pointer;
  transition: background var(--animation-duration, 0.15s) var(--ease-2),
    border-color var(--animation-duration, 0.15s) var(--ease-2),
    color var(--animation-duration, 0.15s) var(--ease-2);
  font-family: inherit;
  text-align: start;
}

.sidebar-new-btn:hover {
  background: var(--surface-3);
  color: var(--text-1);
  border-color: var(--surface-4);
}

.session-title {
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar-empty {
  font-size: var(--font-size-0);
  color: var(--text-2);
  padding: var(--size-2) var(--size-3);
  font-style: italic;
}

/* --- Transitions --- */

.slide-up-enter-active,
.slide-up-leave-active {
  transition: all var(--animation-duration, 0.15s) var(--ease-2);
}

.slide-up-enter-from,
.slide-up-leave-to {
  opacity: 0;
  transform: translateY(var(--size-2));
}
</style>
