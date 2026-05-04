# Frontend Architecture

This document describes the frontend architecture, including pages, components, composables, and styling conventions.

## Overview

The frontend is built with Vue 3 + Vite + Vue Router, running inside a Tauri webview. There is no Nuxt — routing is manual via Vue Router, and components require explicit imports.

### Key Technologies

- **Vue 3**: Progressive JavaScript framework (Composition API)
- **Vite**: Build tool and dev server
- **Vue Router**: Client-side routing
- **tauri-plugin-sql**: Local SQLite for gameplay state
- **Open Props**: CSS custom property design tokens

### Architecture

All gameplay runs client-side inside the Tauri webview:

- **Tauri IPC** (`invoke`) — LLM proxy, file operations, model configuration
- **Local SQLite** (`select` / `execute`) — vignettes, pages, module runtime, profiles
- **LLM streaming** (`useLlmStream`) — text generation via Tauri events

## Pages

Pages are located in `gui/src/pages/` and registered manually in `gui/src/router/index.ts`.

### Home Page (`gui/src/pages/index.vue`)

The home page shows a hero section, the most recent vignettes, and an empty state when none exist.

**Route:** `/`

**Features:**
- Hero section with app title, subtitle, and a "+ New vignette" button linking to `/vignettes/new`
- Recent vignettes section showing up to 3 most recent vignettes as clickable rows (via `useVignetteList`)
- "View all vignettes" link to `/vignettes`
- Empty state message when no vignettes exist

### Vignette Pages

Vignettes are purely client-side — they use local SQLite.

#### Vignette List (`gui/src/pages/vignettes/index.vue`)

Displays the user's local vignette sessions.

**Route:** `/vignettes`

**Data source:** Reads from local SQLite (`local_sessions` table via `useVignetteList`)

#### Vignette Play (`gui/src/pages/vignettes/[id].vue`)

The main vignette gameplay page.

**Route:** `/vignettes/:id` (also supports `/vignettes/new` for creating new vignettes)

**Data source:** Reads/writes to local SQLite (`local_sessions`, `local_pages`, `local_module_runtime`)

**Features:**
- Supports both `new` (from home page "New vignette" button) and existing session IDs
- LLM streaming via `useLlmStream` composable

### Story Builder (`gui/src/pages/builder.vue`)

Story creation and editing interface.

**Route:** `/builder`

### Settings (`gui/src/pages/settings.vue`)

Full page for configuring LLM models. Loads models on mount via `invoke('list_models')`, supports add/edit/delete with Model ID, Base URL, and API Key fields. Persists changes immediately via `invoke('save_models', { models })`. Delete confirmation uses a Teleport overlay.

**Route:** `/settings`

### Creating New Pages

Create a new `.vue` file in `gui/src/pages/` and register it in `gui/src/router/index.ts`:

```typescript
// gui/src/router/index.ts
import MyPage from '~/pages/my-page.vue';

const routes = [
  { path: '/my-page', name: 'my-page', component: MyPage },
  // or use dynamic imports:
  { path: '/my-page', name: 'my-page', component: () => import('~/pages/my-page.vue') },
];
```

## Composables

Composables are located in `gui/src/composables/`. Vue composition API functions (`ref`, `computed`, `watch`, etc.) and vue-router functions (`useRoute`, `useRouter`) are auto-imported at build time by a custom Vite plugin (`gui/vite-plugins/auto-import.ts`). The `select()` and `execute()` local DB helpers are declared in `gui/src/env.d.ts`.

### useLocalDb

Wraps `tauri-plugin-sql` for local SQLite access.

**Location:** `gui/src/composables/useLocalDb.ts`

**Exports:** `select<T>()`, `execute()` (auto-imported globally)

```typescript
const sessions = await select<{ id: string; title: string }>(
  'SELECT id, title FROM local_sessions ORDER BY created_at DESC'
);
```

### useProfiles

Wraps the `local_profiles` SQLite table for managing player profiles.

**Location:** `gui/src/composables/useProfiles.ts`

**Returns:** `{ profiles, activeProfile, refresh, create, update, remove, setActive, init, maxProfiles, defaultFields }`

- `profiles` — `readonly` reactive array of all profiles
- `activeProfile` — computed reference to the profile with `active: true`
- `init()` — loads profiles from local DB, auto-creates a "Default" profile if none exist
- `create(name, fields?)` — creates a new profile (max 5)
- `update(id, patch)` — updates name and/or fields
- `remove(id)` — deletes a profile; if it was active, activates the next available
- `setActive(id)` — sets exactly one profile as active

**Default fields:** `{ name, appearance, interests, favorite color }`

### useLlmStream

Centralized LLM streaming client wrapping Tauri event emission.

**Location:** `gui/src/composables/useLlmStream.ts`

**Exports:**

- `streamLlm(options)` — async generator yielding text chunks only
- `streamLlmFull(options)` — async generator yielding full `StreamEvent` objects (`text`, `reasoning`, `error`, `done`)

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';

for await (const event of streamLlmFull({ persona: PERSONA_PLATFORM, messages })) {
  if (event.type === 'text') { /* append text */ }
  if (event.type === 'reasoning') { /* append reasoning */ }
  if (event.type === 'error') { /* handle error */ }
  if (event.type === 'done') { /* stream complete */ }
}
```

**Rule:** Never listen to Tauri `llm:*` events directly — always use `useLlmStream`.

**Implementation notes:**

The `invoke('prompt', ...)` call to the Rust backend is **fire-and-forget** (the promise is not awaited). This is necessary because the Rust command does not resolve until *after* it has emitted `llm:done`, meaning the `llm:done` event would arrive before the polling loop could start if the invoke were awaited.

The flow is:

1. Event listeners for `llm:text`, `llm:reasoning`, `llm:error`, and `llm:done` are registered.
2. `invoke('prompt', ...)` is called without awaiting. Its `.catch()` captures any Rust-side rejection into `invokeError` and sets `done = true`.
3. A polling loop starts immediately, draining the text/reasoning queues and yielding events. Error events cause an early return.
4. When `done` becomes `true` (via `llm:done` event or invoke rejection), the loop exits.
5. After the loop: if `invokeError` was captured, an `error` event is yielded; otherwise a `done` event is yielded.
6. All listeners are cleaned up in a `finally` block.

### useStoryBuilder

Story builder logic, imports gameplay modules.

**Location:** `gui/src/composables/useStoryBuilder.ts`

Uses `getAllModules()` from `~/gameplay`.

### useVignetteList

Vignette list data and CRUD operations.

**Location:** `gui/src/composables/useVignetteList.ts`

### useShortcutsDialog

Shared state composable for the keyboard shortcuts dialog.

**Location:** `gui/src/composables/useShortcutsDialog.ts`

**Returns:** `{ open: Readonly<Ref<boolean>>, show: () => void, close: () => void, toggle: () => void }`

### useToast

Toast notification system.

**Location:** `gui/src/composables/useToast.ts`

## Components

Reusable Vue components are located in `gui/src/components/` and require **explicit imports**.

### StoryCard

Displays story information in a card format.

**Location:** `gui/src/components/StoryCard.vue`

### AppSidebar

Application navigation sidebar.

**Location:** `gui/src/components/AppSidebar.vue`

### AccountBox

User account section displayed in the sidebar footer. Contains a dropdown menu with items for Profiles, Settings, and Shortcuts. Mounts `ProfilesDialog` internally; the Settings menu item navigates to `/settings` via `router.push`.

**Location:** `gui/src/components/AccountBox.vue`

**Props:** `{ expanded: boolean }`

**Menu items:**
- Profiles — opens `ProfilesDialog`
- Settings — navigates to `/settings`
- Shortcuts — opens the global shortcuts dialog

### ProfilesDialog

Modal dialog for managing player profiles.

**Location:** `gui/src/components/ProfilesDialog.vue`

### SuggestionPicker

Provides AI-generated suggestions during gameplay.

**Location:** `gui/src/components/SuggestionPicker.vue`

Uses `streamLlmFull()` from `useLlmStream.ts`.

### InspireDialog

Inspiration dialog in the story builder.

**Location:** `gui/src/components/builder/InspireDialog.vue`

Uses `streamLlmFull()` from `useLlmStream.ts`.

### Game

Main gameplay component — the single interactive surface for vignette play sessions.

**Location:** `gui/src/components/Game.vue`

### ChatArea

Chat/conversation display area.

**Location:** `gui/src/components/ChatArea.vue`

### GameDebugPanel

Debug panel for gameplay state inspection.

**Location:** `gui/src/components/GameDebugPanel.vue`

### ShortcutsDialog

Modal dialog listing all keyboard shortcuts, mounted in `App.vue`.

**Location:** `gui/src/components/ShortcutsDialog.vue`

### ShortcutRow

Presentational component that renders a keyboard shortcut as a row with a label and styled `<kbd>` keys.

**Location:** `gui/src/components/ShortcutRow.vue`

### GameSessionCard

Vignette/session card for the list view.

**Location:** `gui/src/components/GameSessionCard.vue`

### Creating New Components

Create a new `.vue` file in `gui/src/components/`:

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

**Import Usage:**

```vue
<script setup lang="ts">
import MyComponent from '~/components/MyComponent.vue';
</script>

<template>
  <div>
    <MyComponent title="Hello" subtitle="World" />
  </div>
</template>
```

## Global Styles

Global CSS is defined in `gui/src/assets/css/` and imported in `gui/src/main.ts`.

**Included Styles:**
- Open Props design tokens
- CSS normalize/reset
- Semantic tokens for dark mode (`--text-1`, `--surface-1`, etc.)
- Brand gradient (`--brand-gradient`)

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
<style scoped>
.my-component {
  padding: var(--size-3);
}
</style>
```

### CSS Class Naming

Use kebab-case for CSS class names:

```css
/* Good */
.story-card {}
.cover-art {}
.author-name {}

/* Avoid PascalCase or camelCase for classes */
.StoryCard {}    /* Avoid */
.coverArt {}     /* Avoid */
```

### Logical Properties

Use logical properties for internationalization support:

```css
.inline-size: 100%;
.block-size: auto;
margin-block-start: var(--size-2);
padding-inline: var(--size-3);
```

## Auto-Imports

### Vue Composition API & Vue Router

Auto-imported at build time by a custom Vite plugin (`gui/vite-plugins/auto-import.ts`, registered at `enforce: 'pre'` in `gui/vite.config.ts`). Type declarations live in `gui/src/env.d.ts` for TypeScript support.

**Never manually import these — it causes duplicate import conflicts:**

- From `vue`: `ref`, `reactive`, `computed`, `watch`, `readonly`, `onMounted`, `onUnmounted`, `nextTick`
- From `vue-router`: `useRoute`, `useRouter`

```typescript
<script setup lang="ts">
// No import needed — injected by the Vite auto-import plugin
const count = ref(0);
const route = useRoute();
const router = useRouter();
</script>
```

### Local DB Helpers

Declared in `gui/src/env.d.ts` (type-only declarations; runtime provided by `tauri-plugin-sql`):

```typescript
// No import needed
const sessions = await select<{ id: string }>('SELECT id FROM local_sessions');
await execute('DELETE FROM local_sessions WHERE id = ?', [id]);
```

### Components

Components are **not** auto-imported — import explicitly:

```typescript
import StoryCard from '~/components/StoryCard.vue';
```

## Keyboard Shortcuts

### Global Shortcuts

Registered in `App.vue` on `document`.

| Shortcut | Behavior |
|----------|----------|
| `Ctrl+Shift+/` (also `Ctrl+Shift+?`) | Open/toggle the shortcuts dialog |
| `Alt+N → H` | Navigate to Home (`/`) |
| `Alt+N → V` | Navigate to Vignettes (`/vignettes`) |

### Vignettes List Shortcuts

Registered in `gui/src/pages/vignettes/index.vue` on `document` via `onMounted`.

| Shortcut | Behavior |
|----------|----------|
| `j` or `↓` | Select next vignette |
| `k` or `↑` | Select previous vignette |
| `Enter` | Open the selected vignette |

## Related Documentation

- [Project Structure](./project-structure.md) - File organization for frontend code
- [Code Conventions](./code-conventions.md) - Component patterns and styling guidelines
- [API Routes](./api-routes.md) - Tauri commands used by the frontend
- [Database Schema](./database-schema.md) - Local SQLite schema for gameplay state
