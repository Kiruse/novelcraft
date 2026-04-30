# Frontend Architecture

This document describes the frontend architecture, including pages, components, composables, and styling conventions.

## Overview

The frontend is built with Nuxt 4 and Vue 3, using file-based routing and auto-imports for a streamlined development experience.

### Key Technologies

- **Nuxt 4**: Vue framework with server-side rendering
- **Vue 3**: Progressive JavaScript framework
- **File-based routing**: Pages become routes automatically
- **Auto-imports**: Components, composables, and utilities are auto-imported
- **PowerSync + Drizzle SQLite**: Client-side local database for gameplay state
- **Open Props**: CSS custom property design tokens

### Architecture

Gameplay (vignettes, sessions, module runtime) runs entirely on the client:

- **Server API** (`useFetch`) — auth, user data, shareable story metadata
- **Local SQLite** (`useLocalDb`) — vignettes, pages, module runtime
- **LLM streaming** (`useLlmStream`) — text generation via `POST /api/llm/prompt`

## Pages

Pages are located in `app/pages/` and automatically become routes based on their file path.

### Discovery Page (`app/pages/index.vue`)

The home page displays available stories and vignette quick-start options.

**Route:** `/`

**Features:**
- Displays all stories in a responsive grid (ordered alphabetically by title)
- Vignette quick-start navigates to `/vignettes/new?disposition=...` instead of calling a server API
- Sessions section has been removed (gameplay is client-side now)

**API Calls:**
- `GET /api/stories` - Fetch all stories

**Data Flow:**

```typescript
const { data: stories } = await useFetch('/api/stories');
```

### Vignette Pages

Vignettes are purely client-side — they use local SQLite instead of server API.

#### Vignette List (`app/pages/vignettes/index.vue`)

Displays the user's local vignette sessions.

**Route:** `/vignettes`

**Data source:** Reads from local SQLite (`local_sessions` table via `useLocalDb`)

#### Vignette Play (`app/pages/vignettes/[id].vue`)

The main vignette gameplay page.

**Route:** `/vignettes/:id` (also supports `/vignettes/new` for creating new vignettes)

**Data source:** Reads/writes to local SQLite (`local_sessions`, `local_pages`, `local_module_runtime`)

**Features:**
- Supports both `new` (from home page quick-start) and existing session IDs
- LLM streaming via `useLlmStream` composable

### Story Builder (`app/pages/builder.vue`)

Story creation and editing interface.

**Route:** `/builder`

### Story Play Page (`/stories/:author/:id`)

Displays a published story. Server session endpoints have been removed, so this page is temporarily stubbed.

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

## Composables

Composables are located in `app/composables/` and are auto-imported.

### useLocalDb

Wraps PowerSync client with Drizzle ORM for local SQLite access.

**Location:** `app/composables/useLocalDb.ts`

**Returns:** Drizzle ORM database instance bound to local SQLite

```typescript
const db = useLocalDb();
const sessions = await db.select().from(localSessions);
```

### useProfiles

Wraps the `local_profiles` SQLite table for managing player profiles.

**Location:** `app/composables/useProfiles.ts`

**Exports:** `ProfileRow`, `Profile` types

**Returns:** `{ profiles, activeProfile, refresh, create, update, remove, setActive, init, maxProfiles, defaultFields }`

- `profiles` — `readonly` reactive array of all profiles
- `activeProfile` — computed reference to the profile with `active: true`
- `init()` — loads profiles from local DB, auto-creates a "Default" profile if none exist
- `create(name, fields?)` — creates a new profile (max 5)
- `update(id, patch)` — updates name and/or fields
- `remove(id)` — deletes a profile; if it was active, activates the next available
- `setActive(id)` — sets exactly one profile as active

**Default fields:** `{ name, appearance, interests, favorite color }`

```typescript
const { profiles, activeProfile, init } = useProfiles();
await init();
console.log(activeProfile.value?.fields);
```

### useLlmStream

Centralized SSE streaming client for the LLM proxy endpoint.

**Location:** `app/composables/useLlmStream.ts`

**Exports:**

- `streamLlm(options)` — async generator yielding text chunks only
- `streamLlmFull(options)` — async generator yielding full `StreamEvent` objects (`text`, `reasoning`, `error`, `done`)

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';

for await (const event of streamLlmFull({ persona: PERSONA_PLATFORM, messages })) {
  if (event.type === 'text') { /* append text */ }
  if (event.type === 'error') { /* handle error */ }
}
```

**Rule:** Never parse SSE inline in components — always use `useLlmStream`.

### useCurrentUser

Manages current user state and shape types.

**Location:** `app/composables/useCurrentUser.ts`

**Exports:** `UserShape`, `AuthorStoryShape` (vignette and session shapes have been removed — those are client-side now)

### useStoryBuilder

Story builder logic, imports gameplay modules from shared code.

**Location:** `app/composables/useStoryBuilder.ts`

Uses `getAllModules()` from `#shared/gameplay` instead of a server API endpoint.

### useAuthClient

Authentication client wrapper.

**Location:** `app/composables/useAuthClient.ts`

### useToast

Toast notification system.

**Location:** `app/composables/useToast.ts`

## Components

Reusable Vue components are located in `app/components/` and are auto-imported.

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

### AppSidebar

Application navigation sidebar.

**Location:** `app/components/AppSidebar.vue`

**Note:** Vignette section, sessions section, and `createVignette()` have been removed. Gameplay navigation is handled client-side. The account box (footer) is extracted into the `AccountBox` component.

### AccountBox

User account section displayed in the sidebar footer. Shows user avatar/name, current active profile name, and an account menu (Profiles, Settings, Sign out). Contains the `ProfilesDialog`.

**Location:** `app/components/AccountBox.vue`

**Props:** `{ user: UserShape | null; expanded: boolean }`

**Events:** `closeDrawer`

Initialized via `useProfiles().init()` on mount.

### ProfilesDialog

Modal dialog for managing player profiles (create, edit, delete, switch active). Fields are `key: value` pairs in single text inputs. Tab on last non-empty field creates a new one.

**Location:** `app/components/ProfilesDialog.vue`

**Props:** `{ open, profiles, activeProfile, maxProfiles, defaultFields }`

**Events:** `close`, `create`, `update`, `remove`, `setActive`

### SuggestionPicker

Provides AI-generated suggestions during gameplay.

**Location:** `app/components/SuggestionPicker.vue`

Uses `streamLlmFull()` from `useLlmStream.ts` for LLM streaming.

### InspireDialog

Inspiration dialog in the story builder.

**Location:** `app/components/builder/InspireDialog.vue`

Uses `streamLlmFull()` from `useLlmStream.ts` for LLM streaming.

### Game

Main gameplay component.

**Location:** `app/components/Game.vue`

### ChatArea

Chat/conversation display area.

**Location:** `app/components/ChatArea.vue`

### GameDebugPanel

Debug panel for gameplay state inspection.

**Location:** `app/components/GameDebugPanel.vue`

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
  padding: var(--size-3);
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

## Plugins

### PowerSync Plugin

**Location:** `app/plugins/powersync.client.ts`

Initializes PowerSync on client-side only. Required for local SQLite access.

## Global Styles

Global CSS is defined in `app/assets/css/app.css` and included via `nuxt.config.ts`.

**Included Styles:**
- Open Props design tokens
- CSS normalize/reset
- Semantic tokens for dark mode (`--text-1`, `--surface-1`, etc.)
- Brand gradient (`--brand-gradient`)

### Configuring Global Styles

```typescript
// nuxt.config.ts
export default defineNuxtConfig({
  css: ['~/assets/css/app.css'],
});
```

## Styling Conventions

### Open Props Tokens

Reference design tokens via CSS custom properties:

```vue
<style scoped>
.my-component {
  padding: var(--size-3);
  border-radius: var(--radius-2);
  box-shadow: var(--shadow-2);
  color: var(--text-1);
  background: var(--surface-1);
  font-size: var(--font-size-4);
}
</style>
```

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
  padding: var(--size-3);
}

.text {
  color: var(--text-1);
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

### Logical Properties

Use logical properties for internationalization support:

```css
.inline-size: 100%;
.block-size: auto;
margin-block-start: var(--size-2);
padding-inline: var(--size-3);
```

### Responsive Design

Use CSS Grid and Flexbox for responsive layouts:

```vue
<style scoped>
.stories-grid {
  display: grid;
  grid-template-columns: 1fr; /* Mobile */
  gap: var(--size-6);
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

## Auto-Imports

### Components

All components in `app/components/` are auto-imported:

```vue
<!-- No import needed -->
<template>
  <StoryCard :story="story" />
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
const db = useLocalDb();

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
  vite: {
    // PowerSync WASM worker settings
  },
});
```

## Related Documentation

- [Project Structure](./project-structure.md) - File organization for frontend code
- [Code Conventions](./code-conventions.md) - Component patterns and styling guidelines
- [API Routes](./api-routes.md) - Backend endpoints used by the frontend
- [Database Schema](./database-schema.md) - Local SQLite schema for gameplay state
