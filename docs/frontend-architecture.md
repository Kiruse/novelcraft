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

Displays the user's local vignette sessions with keyboard-navigable rows.

**Route:** `/vignettes`

**Data source:** Reads from local SQLite (`local_sessions` table via `useLocalDb`)

**Keyboard navigation:** `j`/`↓` and `k`/`↑` move a `focusedIndex` through the list; `Enter` opens the focused vignette. A document-level `keydown` listener is added in `onMounted` and removed in `onUnmounted`. Each row is a `<NuxtLink>` tracked via `rowRefs`; focused rows get the `vignette-row--focused` class and DOM focus for the `:focus` outline.

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

### useShortcutsDialog

Shared state composable for the keyboard shortcuts dialog.

**Location:** `app/composables/useShortcutsDialog.ts`

**Returns:** `{ open: Readonly<Ref<boolean>>, show: () => void, close: () => void, toggle: () => void }`

- `open` — readonly reactive boolean controlling dialog visibility
- `show()` — opens the dialog
- `close()` — closes the dialog
- `toggle()` — toggles dialog visibility

Used by both `app.vue` (global `Ctrl+Shift+/` shortcut) and `AccountBox.vue` (menu item) to control the same dialog instance.

```typescript
const { open, show, close, toggle } = useShortcutsDialog();
```

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

User account section displayed in the sidebar footer. Shows user avatar/name, current active profile name, and an account menu (Profiles, Shortcuts, Settings, Sign out). Contains the `ProfilesDialog`. The "Shortcuts" menu item opens the shortcuts dialog via `useShortcutsDialog().show()`.

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

Main gameplay component — the single interactive surface for vignette play sessions.

**Location:** `app/components/Game.vue`

**Props:**

```typescript
interface GameProps {
  /** Pre-built page entries rendered in the story body. */
  pages: GamePage[];
  /** Story title shown in the header input. */
  title?: string;
  /** Placeholder for the title input. */
  titlePlaceholder?: string;
  /** Whether the agent is currently streaming a response. */
  streaming?: boolean;
  /** Accumulated streaming text shown during generation. */
  streamText?: string;
}
```

**Events:**

| Event | Payload | Description |
|-------|---------|-------------|
| `prompt` | `{ text: string; mode: InputMode; pageId: number \| string \| null }` | User submitted a message in the chat bar |
| `updateTitle` | `string` | Title input blurred with a new value |
| `updatePage` | `{ pageId: number \| string; response?: string; system?: string \| null }` | User edited a page's prose response or system prompt |

**Input modes** (cycled via Shift+Tab or the mode button): `write`, `steer`, `instruct`

**Exposed:** `setPage(n: number)` — programmatic page navigation (used by parent).

### ChatArea

Chat/conversation display area.

**Location:** `app/components/ChatArea.vue`

### GameDebugPanel

Debug panel for gameplay state inspection.

**Location:** `app/components/GameDebugPanel.vue`

### ShortcutsDialog

Teleport-ed modal dialog listing all keyboard shortcuts, mounted in `app.vue`.

**Location:** `app/components/ShortcutsDialog.vue`

**Features:**
- Search bar to filter shortcuts by name (default mode)
- "By keys" checkbox toggle to filter by key combination instead
- Highlights matching keys or dims non-matching labels based on active filter mode
- Escape key closes the dialog
- Auto-focuses the search input when opened
- Resets filter state on each open

**Shortcut groups** (defined in the `SHORTCUTS` constant):
- **Global** — shortcuts dialog toggle, Alt+N chord navigation (Home, Vignettes)
- **Vignettes list** — j/k/Enter row navigation
- **Game — input focus** — Ctrl+↑/↓ focus cycle, Ctrl+Enter chat bar
- **Game — chat bar** — Shift+Tab mode cycle, Enter send, ↑ edit last response
- **Game — editors** — Escape close, Ctrl+Enter save
- **Game — slash commands** — /write, /steer, /instruct

Uses `ShortcutRow` with the `highlight` prop to visually emphasize matching elements.

### ShortcutRow

Presentational component that renders a keyboard shortcut as a row with a label and styled `<kbd>` keys.

**Location:** `app/components/ShortcutRow.vue`

**Props:**

```typescript
{
  keys: string;                // Space-separated key tokens, e.g. "Ctrl Shift /"
  label: string;               // Human-readable description of what the shortcut does
  highlight?: 'keys' | 'label' | '';  // Controls visual emphasis when filtering (default: '')
}
```

The `keys` string is split on spaces to produce individual `<kbd>` elements. When `highlight` is `'keys'`, the kbd elements are visually emphasized; when `'label'`, the label text is emphasized. Used by `ShortcutsDialog`.

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

## Keyboard Shortcuts

### Global Shortcuts

Registered in `app.vue` on `document`, so they work on every page.

| Shortcut | Behavior |
|----------|----------|
| `Ctrl+Shift+/` (also `Ctrl+Shift+?`) | Open/toggle the shortcuts dialog |
| `Alt+N → H` | Navigate to Home (`/`) |
| `Alt+N → V` | Navigate to Vignettes (`/vignettes`) |

The `Ctrl+Shift+/` listener also matches `?` because pressing `/` with Shift on US keyboards produces `?`. The dialog is a `ShortcutsDialog` component controlled via `useShortcutsDialog().toggle()`.

**Alt+N chord navigation:** Pressing `Alt+N` sets an internal `awaitingAltN` flag. On the next non-Alt keypress, the flag is cleared and the key is looked up in the `ALT_N_ROUTES` map (`{ h: '/', v: '/vignettes' }`). If a match is found, `navigateTo(route)` is called. Unrecognized keys are silently ignored.

### Vignettes List Shortcuts

Registered in `app/pages/vignettes/index.vue` on `document` via `onMounted`, removed on `onUnmounted`. Only active while the vignettes list page is mounted.

| Shortcut | Behavior |
|----------|----------|
| `j` or `↓` | Select next vignette |
| `k` or `↑` | Select previous vignette |
| `Enter` | Open the selected vignette |

Implementation uses a `focusedIndex` ref to track the currently highlighted row and a `rowRefs` array of `<HTMLElement>` template refs. `moveFocus(delta)` clamps the index and calls `.focus()` on the corresponding `<NuxtLink>` element. The focused row receives the `vignette-row--focused` CSS class (background change + slight upward translate) and a `:focus` outline.

### Game Component Shortcuts

The `Game` component registers its own global `keydown` listener on mount (`onGlobalKeydown`) that handles focus cycling and chat bar access. These shortcuts work from any input within the Game component.

### Focus Cycle Shortcuts

Three focus slots exist in top-to-bottom order: **chat input** → **prose editor** → **system prompt editor**.

| Shortcut | Behavior |
|----------|----------|
| `Ctrl+Up` | Cycle focus upward: chat → prose → system. If a target editor is not yet open but has content to edit, it enters edit mode automatically. Unavailable targets (no content, streaming) are skipped. |
| `Ctrl+Down` | Cycle focus downward: system → prose → chat. Same auto-open and skip logic as `Ctrl+Up`. |

### Chat Bar Shortcut

| Shortcut | Behavior |
|----------|----------|
| `Ctrl+Enter` | Focus the chat bar. If the prose editor or system prompt editor is currently active, it saves (blurs) that editor first, then focuses the chat bar. Works from any input in the Game component. |

### Other Shortcuts

| Shortcut | Context | Behavior |
|----------|---------|----------|
| `Shift+Tab` | Chat input focused | Cycle input mode (write → steer → instruct → write) |
| `ArrowUp` | Chat input empty, response exists | Open prose editor for the current page |
| `Escape` | Prose or system editor focused | Save (blur) the editor |
| `/command ` | Chat input | Slash commands: `/steer`, `/remind`, `/write`, `/instruct` switch mode and strip the command prefix |

### Implementation Details

- The global listener (`onGlobalKeydown`) is registered on `document` in `onMounted` and cleaned up in `onUnmounted`.
- `cycleFocus(direction)` determines the currently focused slot by comparing `document.activeElement` against the template refs (`chatField`, `editArea`, `editSystemArea`), then walks the `FOCUS_CYCLE` array in the requested direction.
- `focusSlot(slot)` returns a boolean indicating whether the target accepted focus — editors that cannot be activated (no content, currently streaming) return `false` so the cycle continues to the next slot.
- The inline `@keydown.ctrl.enter` handler that was previously on the prose editor textarea has been removed in favor of the single global handler.

## Related Documentation

- [Project Structure](./project-structure.md) - File organization for frontend code
- [Code Conventions](./code-conventions.md) - Component patterns and styling guidelines
- [API Routes](./api-routes.md) - Backend endpoints used by the frontend
- [Database Schema](./database-schema.md) - Local SQLite schema for gameplay state
