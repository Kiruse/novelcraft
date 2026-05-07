# Code Conventions

This document describes the code styling, import patterns, and conventions used throughout Novelcraft.

## File Organization

### Modules and Extensions

- **TypeScript**: All source files use `.ts` extension
- **Vue Components**: Use `.vue` extension for single-file components
- **Module type**: ESM modules (`"type": "module"` in gui/package.json)

### Export Patterns

**Gameplay Modules** (`gui/src/gameplay/`)
- Use named exports
- Barrel export via `index.ts`
- Modules: `gameplayModule.ts`, `systemPromptModule.ts`, `eventModule.ts`, `npcModule.ts`, `graphMapModule.ts`

```typescript
// gui/src/gameplay/index.ts
export { getAllModules } from './gameplayModule';
export { ... } from './systemPromptModule';
// etc.
```

**Composables** (`gui/src/composables/`)
- Use named exports
- Auto-imported when declared in `gui/src/env.d.ts`

**Utilities** (`gui/src/utils/`)
- Use named exports
- `tauriLanguageModel.ts` — `TauriLanguageModel` class (implements `LanguageModelV3` from `@ai-sdk/provider`) and `createTauriModel()` factory function

## Import Patterns

### Alias Imports

**Always use the `~/` alias — never relative `../` paths.**

| Alias | Resolves to | Use in |
|-------|------------|----------|
| `~/` | `gui/src/` | All frontend code (pages, components, composables, utils, gameplay) |

- **No `@/`, `#shared/`, `#server/`, or `~~/` aliases** — those were removed

### ES Module Imports

All imports use ES module syntax:

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';
import { PERSONA_PLATFORM } from '~/prompts';
import { getAllModules } from '~/gameplay';
```

### Auto-imports

A custom Vite plugin (`gui/vite-plugins/auto-import.ts`) automatically injects Vue composition API and vue-router imports into all `.ts` and `.vue` (`<script setup>`) files at build time. The plugin is registered in `gui/vite.config.ts` at `enforce: 'pre'` (before `@vitejs/plugin-vue`). Type declarations for these identifiers live in `gui/src/env.d.ts` (for TypeScript only — the Vite plugin handles runtime injection).

**Auto-imported identifiers (never import these manually — it causes duplicate import conflicts):**

- From `vue`: `ref`, `reactive`, `computed`, `watch`, `readonly`, `onMounted`, `onUnmounted`, `nextTick`
- From `vue-router`: `useRoute`, `useRouter`
- **Tauri**: `invoke`, `listen` (declared in `env.d.ts`)
- **Local DB**: `select<T>()`, `execute()` (declared in `env.d.ts`)

Components are **not** auto-imported — import explicitly:

```typescript
import StoryCard from '~/components/StoryCard.vue';
```

## Async Functions

Functions that perform I/O operations should be async:

```typescript
export async function loadSessions(): Promise<Session[]> {
  const sessions = await select<Session>('SELECT * FROM local_sessions');
  return sessions;
}
```

### Async/Await Usage

Always use `async/await` for async operations, avoid `.then()` chains:

```typescript
// Good
const sessions = await select<Session>('SELECT * FROM local_sessions');

// Avoid
select<Session>('SELECT * FROM local_sessions').then(sessions => {
  // Nested callbacks
});
```

## Error Handling

### Throwing Errors

Functions should throw descriptive errors with clear messages:

```typescript
if (!session) {
  throw new Error("Session not found");
}
```

### Error Messages Guidelines

- Be specific about what went wrong
- Include context (what was expected vs what was found)
- Avoid technical jargon in user-facing errors

## Type Safety

### TypeScript Best Practices

- Use explicit return types for exported functions
- Leverage inferred types for internal functions
- Define interfaces for complex objects
- **Never use `as any`** — fix the root cause instead

```typescript
// Good - explicit return type for exported function
export async function loadSessions(): Promise<Session[]> {
  return await select<Session>('SELECT * FROM local_sessions');
}

// Interface for complex objects
export interface ProfileFields {
  name: string;
  appearance: string;
  interests: string;
  [key: string]: string;
}
```

## LLM Streaming Pattern

All LLM streaming must go through `useLlmStream` — never listen to Tauri events directly:

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';
import { PERSONA_PLATFORM } from '~/prompts';

const messages = [
  { author: 'system', content: 'You are a storyteller.' },
  { author: 'user', content: 'Tell me a story.' },
];

for await (const event of streamLlmFull({ persona: PERSONA_PLATFORM, messages })) {
  if (event.type === 'text') {
    // Append text chunk
  }
  if (event.type === 'error') {
    // Handle error
  }
}
```

## Database Query Patterns

### Local Database (SQLite via tauri-plugin-sql)

```typescript
// select() and execute() are auto-imported globally

// Query sessions
const sessions = await select<{ id: string; title: string }>(
  'SELECT id, title FROM local_sessions ORDER BY created_at DESC'
);

// Query with parameters
const pages = await select<{ id: string; response: string }>(
  'SELECT id, response FROM local_pages WHERE session_id = ? ORDER BY created_at',
  [sessionId]
);

// Insert
await execute(
  'INSERT INTO local_pages (id, session_id, system, prompt, response, created_at) VALUES (?, ?, ?, ?, ?, ?)',
  [id, sessionId, system, prompt, response, new Date().toISOString()]
);

// Update
await execute(
  'UPDATE local_sessions SET title = ?, updated_at = ? WHERE id = ?',
  [newTitle, new Date().toISOString(), sessionId]
);

// Delete
await execute('DELETE FROM local_sessions WHERE id = ?', [sessionId]);
```

## Rust Backend Formatting

The Rust backend (`engine/src/`) uses [`rustfmt`](https://rust-lang.github.io/rustfmt/) with a project-level config at `engine/src/rustfmt.toml`:

- **Indentation**: 2 spaces (`tab_spaces = 2`)
- Format with `just fmt`, verify with `just fmt-check`

## Related Documentation

- [Project Structure](./project-structure.md) - File organization and directory layout
- [Database Schema](./database-schema.md) - Table definitions and query patterns
- [API Routes](./api-routes.md) - Tauri command documentation
- [Frontend Architecture](./frontend-architecture.md) - Pages, components, and styling conventions
