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
- LLM streaming via `useGame` composable (which uses `useLlmStream` internally)
- `useGame` exposes `streamText`, `thoughts`, `tokenUsage`, `prompt`, and `status` — `thoughts` accumulates reasoning-delta events, `tokenUsage` holds the latest `LlmUsage` from the stream's done event, `prompt` holds a `PromptDebug` snapshot of the last LLM request
- Passes `thoughts`, `tokenUsage`, `prompt` (as `prompt-debug`), and `debugMode` (from `useDebugMode`) to the `Game` component for the debug panel
- Passes the session's `disposition` to the `Game` component, which renders it as a read-only page 0
- Page creation respects the user's current viewing position: `createPage(prompt, pageIndex)` calls `vignette.fork()` to truncate all subsequent pages before appending the new one when the user submits a "write" prompt from a non-last page. This branches the story at that point rather than always appending to the end

### Story Builder (`gui/src/pages/builder.vue`)

Story creation and editing interface.

**Route:** `/builder`

### Settings (`gui/src/pages/settings.vue`)

Settings page that renders `<ModelsConfigurator />` and a "Debug Zone" section. The Debug Zone contains a "Debug Mode" checkbox (backed by `useDebugMode`, persisted to localStorage) and a "Reset Onboarding" button.

**Route:** `/settings`

### Onboarding (`gui/src/Onboarding.vue`)

Step-based first-run onboarding flow rendered by `App.vue` when onboarding is not yet complete.

**Location:** `gui/src/Onboarding.vue`

**Steps:**

| Step | Content |
|------|---------|
| 0 | Welcome page — app description and alpha disclaimer (yellow left-border callout) |
| 1 | Model configuration — renders `<ModelsConfigurator />` |

**Features:**
- Footer with step dot indicators, Back/Next/Get Started buttons
- Fade transitions between steps
- Completing onboarding calls `useOnboarding().complete()` and transitions to the main app

### Creating New Pages

Page creation in the Vignette Play page uses `createPage(prompt, pageIndex)`. When `pageIndex` points to a non-last page, the function calls `vignette.fork()` at that index to truncate all subsequent pages in the database before pushing the new page. This branches the story at the user's current viewing position instead of always appending to the end.

### App Root (`gui/src/App.vue`)

Top-level component that switches between onboarding and the main application. Uses top-level await on `useOnboarding()` to check completion state. Renders `<Onboarding />` if not complete, `<Main />` otherwise.

### Main Shell (`gui/src/Main.vue`)

Contains the previous `App.vue` content: sidebar, router view, global keyboard shortcuts (`Ctrl+Shift+/`, `Alt+N` navigation), and global dialogs (`ShortcutsDialog`, `HostLivenessDialog`).

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

Centralized LLM streaming client built on `ConversationalArchetype` from `@stegakir/aikit` and `createTauriModel` from `~/utils/tauriLanguageModel`.

**Location:** `gui/src/composables/useLlmStream.ts`

**Exports:**

- `streamLlm(options)` — async generator yielding text chunks only
- `streamLlmFull(options)` — async generator yielding full `StreamEvent` objects (`text`, `reasoning`, `error`, `done`)
- `LlmUsage` — token usage interface (`prompt_tokens`, `completion_tokens`, `total_tokens`)
- `LlmDonePayload` — done event payload interface (`finish_reason`, `usage?`)

**Options (`StreamLlmOptions`):**

```typescript
{
  persona: string;
  messages: Array<{ author: string; content: string }>;
  model?: string;        // Usage ID (defaults to DEFAULT_MODEL from ~/prompts, currently 'storyteller')
  context?: Record<string, unknown>;  // passed to ConversationalArchetype as context
}
```

**StreamEvent fields:**

```typescript
{
  type: 'text' | 'reasoning' | 'done' | 'error';
  data: string;
  finishReason?: string;   // populated on 'done' events
  usage?: LlmUsage;        // populated on 'done' events
}
```

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';

for await (const event of streamLlmFull({ persona: PERSONA_PLATFORM, messages })) {
  if (event.type === 'text') { /* append text */ }
  if (event.type === 'reasoning') { /* append reasoning */ }
  if (event.type === 'error') { /* handle error */ }
  if (event.type === 'done') { /* event.finishReason, event.usage available */ }
}
```

**Rule:** Never listen to Tauri `llm:*` events directly — always use `useLlmStream`. For AI SDK integration, use `createTauriModel()` from `~/utils/tauriLanguageModel` instead.

**Internal architecture:**

1. A `MemoryMessageStore` and `Conversation` are created from the incoming `messages` array (each message is pushed via `message()` from `@stegakir/aikit`).
2. A `ConversationalArchetype` is instantiated with the `persona` and optional `context`.
3. `archetype.prompt()` is called with `createTauriModel(modelId)` as the model and the conversation. This triggers `TauriLanguageModel.doStream()` which internally manages Tauri IPC (event listeners, request ID scoping, SSE parsing).
4. The resulting `TextStreamPart` stream is mapped to `StreamEvent` objects: `text-delta` → `{ type: 'text', data }`, `reasoning-delta` → `{ type: 'reasoning', data }`, `finish` → `{ type: 'done', data: '', finishReason, usage }`, `error` → `{ type: 'error', data }`.
5. Tool-related stream parts (`tool-call`, `tool-result`) are handled transparently by `ConversationalArchetype`'s internal `ToolLoopAgent` — callers never see them.
6. Request ID scoping for concurrent streams is managed internally by `TauriLanguageModel.doStream()` — no `requestId` parameter is exposed to callers.

### useStoryBuilder

Story builder logic, imports gameplay modules.

**Location:** `gui/src/composables/useStoryBuilder.ts`

Uses `getAllModules()` from `~/gameplay`.

### useVignetteList

Vignette list data and CRUD operations.

**Location:** `gui/src/composables/useVignetteList.ts`

### useVignette

Manages a single vignette session — its metadata, pages, and database mutations.

**Location:** `gui/src/composables/useVignette.ts`

**Returns:** `{ status, meta, pages, error, save, push, fork }`

- `status` — `Readonly<Ref<'loading' | 'ready' | 'error'>>`
- `meta` — `Ref<VignetteMeta>` — reactive session metadata (`title`, `disposition`, `createdAt`, `updatedAt`)
- `pages` — `Readonly<Ref<VignettePage[]>>` — reactive page array
- `error` — `Readonly<Ref<string | undefined>>`
- `save()` — persists `meta` changes to `local_sessions`
- `push(prompt?)` — appends a new page to the end (DB insert + reactive array push); the new page has no response yet
- `fork({ pageIndex, system?, prompt?, response? })` — updates a page in-place and truncates all pages after `pageIndex`. Deletes the truncated pages from `local_pages` in SQLite and replaces the in-memory array. Used by the Vignette Play page to branch the story when the user writes from a non-last page

**Fork semantics:** `fork()` is the mechanism for branching/rewriting story history. It atomically (within a SQLite transaction) updates the target page's fields and deletes all subsequent pages. After the transaction, the reactive `pages` array is replaced with the truncated list.

### useGame

Manages the LLM gameplay loop for a vignette session — builds messages, streams the response, and exposes reactive state for the UI.

**Location:** `gui/src/composables/useGame.ts`

**Exported types:**

- `GameStatus` — `'idle' | 'streaming'`
- `PromptDebug` — debug snapshot of the last LLM prompt sent:
  ```typescript
  interface PromptDebug {
    persona: string;
    messages: readonly { author: string; content: string }[];
    model?: string;
    context?: Record<string, unknown>;
    promptId?: string;
  }
  ```
- `UseGameOpts` — options for `useGame()` (`meta`, `pages`)
- `GameRunOpts` — options for `run()` (`promptId?`, `getMessages?`, `prependStreamText?`)

**Returns:** `{ status, streamText, thoughts, tokenUsage, prompt, run }`

- `status` — `Readonly<Ref<GameStatus>>`
- `streamText` — accumulated text content from the LLM stream (cleared when streaming starts and ends)
- `thoughts` — accumulated reasoning text from reasoning-delta events (cleared when streaming starts)
- `tokenUsage` — latest token usage (`LlmUsage`) from the stream's done event (prompt_tokens, completion_tokens, total_tokens)
- `prompt` — `Readonly<Ref<PromptDebug | undefined>>` — snapshot of the last prompt sent to the LLM (populated during streaming, includes persona, messages, model, context, and promptId)
- `run(opts?)` — builds messages via `buildMessages()`, streams via `streamLlmFull()`, accumulates text and reasoning chunks. Accepts optional `promptId` (for debug identification), `getMessages` (function to mutate the messages array before sending, used for steer/inject modes), and `prependStreamText` (text to prepend to the live stream output)

### useShortcutsDialog

Shared state composable for the keyboard shortcuts dialog.

**Location:** `gui/src/composables/useShortcutsDialog.ts`

**Returns:** `{ open: Readonly<Ref<boolean>>, show: () => void, close: () => void, toggle: () => void }`

### useHostLiveness

Singleton composable that checks which configured LLM hosts are unreachable. Uses module-level refs so all callers share the same state.

**Location:** `gui/src/composables/useHostLiveness.ts`

**Returns:** `{ unreachableHosts, checked, checkHosts, isHostUnreachable, dismiss }`

- `unreachableHosts` — `Readonly<Ref<UnreachableHost[]>>` — list of hosts that failed the liveness check
- `checked` — `Readonly<Ref<boolean>>` — whether at least one check has been performed
- `checkHosts()` — calls `invoke('ping_hosts')` and populates `unreachableHosts`
- `isHostUnreachable(baseUrl: string)` — returns `true` if the given URL is in the unreachable list
- `dismiss()` — clears the unreachable hosts list

```typescript
interface UnreachableHost {
  url: string;
  error: string;
}
```

### useOnboarding

Singleton composable that tracks whether first-run onboarding has been completed. Uses module-level state so all callers share the same instance.

**Location:** `gui/src/composables/useOnboarding.ts`

**Usage:** `await useOnboarding()` — must be awaited because it queries SQLite on first call.

**Returns:** `{ completed: Readonly<Ref<boolean>>, complete: () => Promise<void> }`

- `completed` — `true` once onboarding is done (checked against `local_onboarding` table on first call)
- `complete()` — writes `completed = 1` to `local_onboarding` and sets the reactive ref to `true`

### useToast

Toast notification system.

**Location:** `gui/src/composables/useToast.ts`

### useDebugMode

Singleton composable that provides a `debugMode` boolean ref persisted to localStorage. When enabled, the Game component renders a `DebugPanel` on the right side showing the LLM's accumulated reasoning ("Thoughts") and token usage ("Token Usage"). All callers share the same instance.

**Location:** `gui/src/composables/useDebugMode.ts`

**Returns:** `{ debugMode: Ref<boolean> }`

- `debugMode` — reactive boolean, initialized from `localStorage` key `novelcraft:debugMode` on mount, auto-saved on change
- Toggled via a checkbox on the Settings page under the "Debug Zone" section

## Utilities

### TauriLanguageModel

Bridges the Tauri LLM proxy to the Vercel AI SDK (`@ai-sdk/provider`) by implementing the `LanguageModelV3` interface.

**Location:** `gui/src/utils/tauriLanguageModel.ts`

**Exports:**

- `TauriLanguageModel` — class implementing `LanguageModelV3`
- `createTauriModel(modelId: string): LanguageModelV3` — factory function

**Purpose:** Enables using `ConversationalArchetype` from `@stegakir/aikit` with the Tauri LLM proxy. Used internally by `useLlmStream` — callers typically do not need to import this directly.

**How it works:**

- `doStream()`: Converts V3 prompt messages to Tauri format, generates a scoped `requestId` (UUID), registers scoped Tauri event listeners (`llm:text:{requestId}`, etc.), creates a `ReadableStream<LanguageModelV3StreamPart>` that bridges Tauri events to V3 stream parts. Handles text, reasoning, tool call streaming, finish with usage, errors, and abort signals.
- `doGenerate()`: Collects the full stream and returns a `LanguageModelV3GenerateResult`.
- Emits `-start` then `-delta` pairs (no delta data in start events) as required by the V3 spec.
- Does not pass `persona` — designed for AI SDK callers that provide their own system messages.

**Message conversion:**

| V3 role | Tauri `author` | Notes |
|---------|---------------|-------|
| `system` | `system` | Direct content mapping |
| `user` | `user` | Text parts joined |
| `assistant` | `ai` | Text parts + `tool_calls` array |
| `tool` | `tool` | Sets `tool_call_id`, joins result outputs |

**Tool conversion:** Maps `LanguageModelV3FunctionTool` to Tauri's `LlmTool` format (`inputSchema` → `parameters`).

```typescript
import { createTauriModel } from '~/utils/tauriLanguageModel';

const model = createTauriModel('storyteller');
// Use with ConversationalArchetype from @stegakir/aikit
```

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

Main gameplay component — the single interactive surface for vignette play sessions. Supports a virtual "page 0" that renders the session disposition as read-only markdown. When debug mode is enabled (via `useDebugMode`), renders a `DebugPanel` component on the right side displaying the LLM's accumulated reasoning, token usage, and prompt debug data.

**Location:** `gui/src/components/Game.vue`

**Props:**

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `vignette` | `DeepReadonly<Vignette>` | required | Vignette instance (reactive `meta` and `pages`) |
| `titlePlaceholder` | `string` | `'Untitled'` | Placeholder for the title input |
| `streaming` | `boolean` | `false` | Whether an LLM response is currently streaming |
| `streamText` | `string` | `''` | Current streaming text content |
| `thoughts` | `string` | `''` | Accumulated reasoning text from the LLM stream |
| `tokenUsage` | `LlmUsage \| undefined` | `undefined` | Latest token usage from the LLM stream (prompt_tokens, completion_tokens, total_tokens) |
| `promptDebug` | `PromptDebug \| undefined` | `undefined` | Last prompt debug data from `useGame().prompt` |

**Events:**

| Event | Payload | Description |
|-------|---------|-------------|
| `prompt` | `{ text: string; mode: InputMode; pageIndex: number }` | User submitted a message (write/steer/instruct) from the given page index |
| `updateTitle` | `string` | Title input blurred with a new value |
| `updatePage` | `{ pageIndex: number; response?: string; system?: string \| null; prompt?: string }` | User edited a page's response |
| `updatePageIndex` | `number` | User navigated between pages |

The `pageIndex` in the `prompt` event reflects the page the user was viewing when they submitted input. The parent page (`[id].vue`) uses this to decide whether to fork the vignette at that position before creating a new page.

**Disposition page (virtual page 0):**

When `disposition` is a non-empty string, the Game component prepends a virtual page to the page list. This page displays the disposition text as rendered markdown with a subtle "Disposition" label. It is always the first page in the page indicator (`1 / N`) and is navigable, but no editing controls are available on it.

Key computed properties driving this behavior:

- `hasDispositionPage` — `true` when `disposition` has non-whitespace content
- `totalPages` — `pages.length + 1` when disposition page exists, otherwise `pages.length`
- `realPageIndex` — maps the 0-indexed `currentPage` to the actual `pages` array index (offset by -1 when disposition page is present)
- `renderedDisposition` — disposition text rendered to HTML via markdown

Navigation, page indicator, and auto-advance logic all account for the virtual disposition page offset.

### ChatArea

Chat/conversation display area.

**Location:** `gui/src/components/ChatArea.vue`

### GameDebugPanel

Debug panel for gameplay state inspection (module runtime state, knowledge, tools). Separate from `DebugPanel` which shows LLM thoughts/token usage.

**Location:** `gui/src/components/GameDebugPanel.vue`

### DebugPanel

Lightweight debug panel extracted from `Game.vue`. Displays the LLM's accumulated reasoning ("Thoughts"), token usage ("Token Usage"), and prompt debug data ("Prompt") as collapsible sections. Rendered by `Game.vue` when debug mode is enabled (toggled via `useDebugMode`).

**Location:** `gui/src/components/DebugPanel.vue`

**Props:**

| Prop | Type | Description |
|------|------|-------------|
| `thoughts` | `string` | Accumulated reasoning text from the LLM stream |
| `tokenUsage` | `LlmUsage \| undefined` | Latest token usage from the LLM stream |
| `promptDebug` | `PromptDebug \| undefined` | Last prompt debug data from `useGame().prompt` |

Uses the `Collapsible` component internally for all three sections.

**Prompt tree view** (third "Prompt" collapsible section):

When `promptDebug` is populated, renders a tree view displaying:

- `persona` and `model` as key-value branches (rendered by `TreeBranch`)
- `promptId` as `id` key-value branch (rendered by `TreeBranch`)
- `context` as a recursive nested tree (rendered by `TreeObject` — recurses into nested objects, renders leaf values as key-value branches)
- `messages` as a count branch showing `[N]`, expandable to a list of individual messages. Each message shows the `author` label and a truncated content preview (80 chars, newlines collapsed). Clicking a message expands it to show the full content in a `<pre>` block.

`TreeBranch` and `TreeObject` are render-function components defined in the `<script lang="ts">` block (non-setup) using `defineComponent` + `h()`. They are also exported for potential reuse.

### ShortcutsDialog

Modal dialog listing all keyboard shortcuts, mounted in `App.vue`.

**Location:** `gui/src/components/ShortcutsDialog.vue`

### HostLivenessDialog

Modal dialog that automatically checks LLM host liveness on mount. Displays a list of unreachable hosts with their URL and error message. Provides "Dismiss" (clears the unreachable list via `useHostLiveness().dismiss()`) and "Go to Settings" (navigates to `/settings`) buttons. Mounted in `App.vue` alongside `ShortcutsDialog`.

**Location:** `gui/src/components/HostLivenessDialog.vue`

### ShortcutRow

Presentational component that renders a keyboard shortcut as a row with a label and styled `<kbd>` keys.

**Location:** `gui/src/components/ShortcutRow.vue`

### Spinner

Reusable CSS-only loading spinner with size variants and accessible labeling.

**Location:** `gui/src/components/Spinner.vue`

**Props:**

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `size` | `'sm' \| 'md'` | `'sm'` | Spinner size |
| `label` | `string` | `'Loading'` | Accessible `aria-label` for screen readers |

Extracted from the inline `.ping-spinner` that previously lived in the settings page.

### Tooltip

Reusable floating tooltip component built on Floating UI. Renders tooltip content via `<Teleport to="body">` and positions it relative to an anchor element using `computePosition` with `offset` and `shift` middleware. Uses `autoUpdate` for dynamic repositioning while visible.

**Location:** `gui/src/components/Tooltip.vue`

**Note on Floating UI packages:** `@floating-ui/vue` is installed as a dependency, but `Tooltip.vue` imports directly from `@floating-ui/dom` (`computePosition`, `offset`, `shift`, `autoUpdate`) to avoid Vue 3.5 `Ref` type invariance issues with the Vue adapter.

**Props:**

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `anchor` | `() => HTMLElement \| null \| undefined` | required | Getter function returning the target element to position against |
| `visible` | `boolean` | `true` | Controls tooltip visibility |

**Slots:**

| Slot | Description |
|------|-------------|
| default | Tooltip content |

**Behavior:**

- Renders into `<Teleport to="body">` with `pointer-events: none` so it doesn't interfere with pointer events
- Positioning uses `placement: 'bottom-start'` with `offset(4)` and `shift({ padding: 4 })` middleware
- `autoUpdate` is started when `visible` becomes `true` (after `nextTick`) and cleaned up when `visible` becomes `false` or the component unmounts
- Styled with Open Props tokens: red background (`--red-7`), white text (`--gray-0`), small font (`--font-size-0`), compact padding, rounded corners

**Usage:**

```vue
<script setup lang="ts">
import Tooltip from '~/components/Tooltip.vue';
const urlInputRef = ref<HTMLElement | null>(null);
const pingError = ref<string | null>(null);
</script>

<template>
  <input ref="urlInputRef" />
  <Tooltip :anchor="() => urlInputRef" :visible="!!pingError">
    {{ pingError }}
  </Tooltip>
</template>
```

### ModelConfigBox

Encapsulated LLM model configuration card. Handles all edit state, save/cancel logic, and debounced liveness checking for a single model entry. Used by the settings page to render each configured model.

**Location:** `gui/src/components/ModelConfigBox.vue`

**Props:**

| Prop | Type | Description |
|------|------|-------------|
| `id` | `string` | Model usage ID (e.g. `"storyteller"`, `"suggestions"`) |
| `config` | `ModelConfig` | Current model configuration (`model_id`, `base_url`, `api_key?`) |
| `initiallyUnreachable` | `boolean` | Whether the host was unreachable at page load (from `useHostLiveness`) |

**Events:**

| Event | Payload | Description |
|-------|---------|-------------|
| `save` | `ModelConfig` | Emitted when the user saves an edited model configuration |

**Behavior:**

- Uses `v-model` for expanded/collapsed state (wraps `Collapsible` internally)
- Internally manages `pinging` and `pingError` refs for liveness checking
- On `base_url` change, a debounced (1s) ping runs via `invoke('ping_host')`
- Shows a red border on the URL input when `pingError` is set
- Shows a `Tooltip` below the Base URL input when `pingError` is set, displaying the error message
- Shows a `Spinner` in front of Save/Cancel buttons while a ping is in flight
- Disables the Save button (`:disabled="pinging"`) while a ping is in progress — the button renders at 50% opacity with a `not-allowed` cursor. Re-enabled once the ping completes regardless of result.
- Shows a red border on the whole box when `initiallyUnreachable` is `true` and no local ping error exists yet
- Edit fields: Model ID, Base URL, API Key (the Usage ID is read-only, used as the HashMap key)

### ModelsConfigurator

Reusable section component for LLM model configuration. Contains the models list with a `ModelConfigBox` for each model, model path display (with a "Your model configuration is saved to:" description prefix followed by the path in monospace font), and save logic. Extracted from `settings.vue` so it can be shared with the onboarding flow.

**Location:** `gui/src/components/ModelsConfigurator.vue`

**Behavior:**
- Loads models on mount via `invoke('list_models')` and saves via `invoke('save_models', { models })`
- The set of usage IDs is fixed (`storyteller`, `suggestions`) and cannot be added or removed
- Each model is rendered as a `ModelConfigBox` component
- The models path hint displays a description prefix "Your model configuration is saved to:" followed by the path in monospace font, with a guaranteed separator between directory and filename
- Used by both `settings.vue` (settings page) and `Onboarding.vue` (step 1)

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

Registered in `Main.vue` on `document`.

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
