# Frontend Architecture

This document describes the frontend architecture, including pages, components, and styling conventions.

## Overview

The frontend is built with Nuxt 4 and Vue 3, using file-based routing and auto-imports for a streamlined development experience.

### Key Technologies

- **Nuxt 4**: Vue framework with server-side rendering
- **Vue 3**: Progressive JavaScript framework
- **File-based routing**: Pages become routes automatically
- **Auto-imports**: Components, composables, and utilities are auto-imported

## Pages

Pages are located in `app/pages/` and automatically become routes based on their file path.

### Discovery Page (`app/pages/index.vue`)

The home page displays available stories and the user's recent game sessions.

**Route:** `/`

**Features:**
- Shows "Jump back in" section with recent game sessions (only if authenticated and has sessions)
- Displays all stories in a responsive grid
- Stories are ordered alphabetically by title
- Empty state handling when no stories exist
- Horizontal scrolling for sessions section

**Responsive Grid:**
- Mobile: 1 column
- Tablet: 2 columns
- Desktop: 3 columns

**API Calls:**
- `GET /api/stories` - Fetch all stories
- `GET /api/sessions` - Fetch user's recent sessions

**Data Flow:**

```typescript
const { data: stories } = await useFetch('/api/stories');
const { data: sessions } = await useFetch('/api/sessions');
```

**Conditional Rendering:**

```vue
<template>
  <!-- Jump back in section (only if authenticated with sessions) -->
  <section v-if="sessions && sessions.length > 0">
    <h2>Jump back in</h2>
    <div class="sessions-scroll">
      <GameSessionCard
        v-for="session in sessions"
        :key="session.id"
        :session="session"
      />
    </div>
  </section>

  <!-- All stories section -->
  <section>
    <h2>All Stories</h2>
    <div class="stories-grid">
      <StoryCard
        v-for="story in stories"
        :key="story.id"
        :story="story"
      />
    </div>
  </section>
</template>
```

### Story Detail Page (`app/pages/stories/[id].vue`)

Displays detailed information about a specific story.

**Route:** `/stories/:id`

**Dynamic Route Parameter:** `id` - Story ID (integer)

**Usage:**

```typescript
const route = useRoute();
const storyId = route.params.id;

const { data: story } = await useFetch(`/api/stories/${storyId}`);
```

### Creating New Pages

Create a new `.vue` file in `app/pages/`:

```typescript
// app/pages/about.vue
<script setup lang="ts">
const pageTitle = 'About Novelcraft';
</script>

<template>
  <div>
    <h1>{{ pageTitle }}</h1>
    <p>About content here</p>
  </div>
</template>
```

**Dynamic Routes:**

```typescript
// app/pages/users/[id].vue
<script setup lang="ts">
const route = useRoute();
const userId = route.params.id;
</script>

<template>
  <div>User {{ userId }}</div>
</template>
```

## Components

Reusable Vue components are located in `app/components/` and are auto-imported. No manual imports required.

### StoryCard

Displays story information in a card format.

**Location:** `app/components/StoryCard.vue`

**Props:**

```typescript
{
  story: {
    id: number;
    title: string;
    description: string | null;
    coverArt: string | null;
    author: {
      name: string;
    };
  };
}
```

**Features:**
- Shows cover art (if available) with fallback
- Displays title and description
- Shows author name
- Links to story detail page via `/stories/:id`

**Template:**

```vue
<script setup lang="ts">
interface Props {
  story: {
    id: number;
    title: string;
    description: string | null;
    coverArt: string | null;
    author: {
      name: string;
    };
  };
}

const props = defineProps<Props>();
</script>

<template>
  <NuxtLink :to="`/stories/${story.id}`" class="story-card">
    <div class="cover">
      <img v-if="story.coverArt" :src="story.coverArt" :alt="story.title" />
      <div v-else class="placeholder">No cover</div>
    </div>
    <div class="content">
      <h3>{{ story.title }}</h3>
      <p v-if="story.description" class="description">{{ story.description }}</p>
      <p class="author">by {{ story.author.name }}</p>
    </div>
  </NuxtLink>
</template>
```

### GameSessionCard

Displays game session information for the "Jump back in" section.

**Location:** `app/components/GameSessionCard.vue`

**Props:**

```typescript
{
  session: {
    id: number;
    updatedAt: Date;
    story: {
      title: string;
      coverArt: string | null;
      genre: string | null;
    };
  };
}
```

**Features:**
- Shows story cover art
- Displays story title and genre
- Shows "Last played" timestamp

**Template:**

```vue
<script setup lang="ts">
interface Props {
  session: {
    id: number;
    updatedAt: Date;
    story: {
      title: string;
      coverArt: string | null;
      genre: string | null;
    };
  };
}

const props = defineProps<Props>();

const formatDate = (date: Date) => {
  return new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
  }).format(new Date(date));
};
</script>

<template>
  <NuxtLink :to="`/sessions/${session.id}`" class="session-card">
    <div class="cover">
      <img v-if="session.story.coverArt" :src="session.story.coverArt" :alt="session.story.title" />
      <div v-else class="placeholder">No cover</div>
    </div>
    <div class="content">
      <h3>{{ session.story.title }}</h3>
      <p v-if="session.story.genre" class="genre">{{ session.story.genre }}</p>
      <p class="last-played">Last played {{ formatDate(session.updatedAt) }}</p>
    </div>
  </NuxtLink>
</template>
```

### Creating New Components

Create a new `.vue` file in `app/components/`:

```vue
<script setup lang="ts">
interface Props {
  title: string;
  subtitle?: string;
}

defineProps<Props>();
</script>

<template>
  <div class="my-component">
    <h2>{{ title }}</h2>
    <p v-if="subtitle">{{ subtitle }}</p>
  </div>
</template>

<style scoped>
.my-component {
  padding: 1rem;
}
</style>
```

**Auto-import Usage:**

```vue
<!-- No import needed - component is auto-imported -->
<template>
  <div>
    <MyComponent title="Hello" subtitle="World" />
  </div>
</template>
```

## Global Styles

Global CSS is defined in `app/assets/css/app.css` and included via `nuxt.config.ts`.

**Included Styles:**
- CSS reset for consistency
- Base element styling
- Typography defaults
- Layout utilities

### Configuring Global Styles

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  css: ['~/assets/css/app.css'],
});
```

## Styling Conventions

### Scoped Styles

Use scoped styles in single-file components to avoid style conflicts:

```vue
<template>
  <div class="my-component">
    <p class="text">Content</p>
  </div>
</template>

<style scoped>
.my-component {
  padding: 1rem;
}

.text {
  color: #333;
}
</style>
```

### CSS Class Naming

Use kebab-case for CSS class names:

```vue
<style scoped>
/* Good */
.story-card {}
.cover-art {}
.author-name {}

/* Avoid PascalCase or camelCase for classes */
.StoryCard {}    /* Avoid */
.coverArt {}     /* Avoid */
</style>
```

### Responsive Design

Use CSS Grid and Flexbox for responsive layouts:

```vue
<style scoped>
.stories-grid {
  display: grid;
  grid-template-columns: 1fr; /* Mobile */
  gap: 1.5rem;
}

@media (min-width: 768px) {
  .stories-grid {
    grid-template-columns: repeat(2, 1fr); /* Tablet */
  }
}

@media (min-width: 1024px) {
  .stories-grid {
    grid-template-columns: repeat(3, 1fr); /* Desktop */
  }
}
</style>
```

### Breakpoints

Common breakpoints used in the project:

| Breakpoint | Screen Width | Description |
|------------|--------------|-------------|
| Mobile | < 768px | Single column layouts |
| Tablet | 768px - 1023px | Two column layouts |
| Desktop | ≥ 1024px | Three+ column layouts |

### Horizontal Scrolling

For horizontal scroll containers:

```vue
<style scoped>
.sessions-scroll {
  display: flex;
  gap: 1rem;
  overflow-x: auto;
  scroll-snap-type: x mandatory;
  -webkit-overflow-scrolling: touch;
  padding-bottom: 0.5rem;
}

.sessions-scroll::-webkit-scrollbar {
  height: 6px;
}

.sessions-scroll::-webkit-scrollbar-track {
  background: #f1f1f1;
  border-radius: 3px;
}

.sessions-scroll::-webkit-scrollbar-thumb {
  background: #ccc;
  border-radius: 3px;
}

.session-card {
  scroll-snap-align: start;
  flex-shrink: 0;
  width: 280px;
}
</style>
```

## Auto-Imports

### Components

All components in `app/components/` are auto-imported:

```vue
<!-- No import needed -->
<template>
  <StoryCard :story="story" />
  <GameSessionCard :session="session" />
</template>
```

### Composables

Vue and Nuxt composables are auto-imported:

```typescript
<script setup lang="ts">
// No import needed
const count = ref(0);
const route = useRoute();
const router = useRouter();

const { data, pending, error } = await useFetch('/api/stories');
</script>
```

## Nuxt Configuration

The Nuxt configuration is defined in `nuxt.config.ts`:

```typescript
// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2024-11-01',
  devtools: { enabled: true },
  css: ['~/assets/css/app.css'],
});
```

## Related Documentation

- [Project Structure](./project-structure.md) - File organization for frontend code
- [Code Conventions](./code-conventions.md) - Component patterns and styling guidelines
- [API Routes](./api-routes.md) - Backend endpoints used by the frontend
