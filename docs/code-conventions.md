# Code Conventions

This document describes the code styling, import patterns, and conventions used throughout Novelcraft.

## File Organization

### Modules and Extensions

- **TypeScript**: All source files use `.ts` extension
- **Vue Components**: Use `.vue` extension for single-file components
- **Module type**: ESM modules (`"type": "module"` in package.json)

### Export Patterns

**Generators** (`scripts/generators/`)
- Use default exports
- Export a single generator function per file

```typescript
export default defineGenerator(async () => {
  // Generator logic
});
```

**Utilities** (`scripts/utils/`)
- Use named exports
- Centralized exports via `index.ts`

```typescript
export async function getProjectRoot(startDir?: string): Promise<string> {
  // Implementation
}

export function defineGenerator<T>(fn: () => Promise<T>): () => Promise<T> {
  return fn;
}
```

**Gameplay Modules** (`shared/gameplay/`)
- Use named exports
- Barrel export via `index.ts`
- Modules: `gameplayModule.ts`, `systemPromptModule.ts`, `eventModule.ts`, `npcModule.ts`, `graphMapModule.ts`

```typescript
// shared/gameplay/index.ts
export { getAllModules } from './gameplayModule';
export { ... } from './systemPromptModule';
// etc.
```

**Local DB Schema** (`shared/db/`)
- Drizzle SQLite table definitions in `localSchema.ts`
- Barrel export via `index.ts`

## Import Patterns

### Alias Imports

**Always use alias imports — never relative `../` paths.**

| Alias | Resolves to | Use in |
|-------|------------|----------|
| `~/` or `@/` | `app/` | App code (pages, components, composables) |
| `#server/` | `server/` | Server code (API routes, plugins) |
| `#shared/` | `shared/` | Shared code (from both app and server) |
| `~~/` | Project root | Escape hatch (avoid unless necessary) |

**Important:**
- The `#server` import alias is only reliably available from code in `./server`. In `./shared`, use `~~/server` instead.
- In `./app`, neither `#server` nor `~~/server` should ever be used — this WILL BREAK THE APP.

### ES Module Imports

All imports use ES module syntax:

```typescript
import { access, constants } from "fs/promises";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";
```

### Auto-imports

The following are auto-imported by Nuxt and do not require explicit imports:

- **Vue composables**: `ref`, `computed`, `onMounted`, etc.
- **Nuxt utilities**: `useFetch`, `useRoute`, `navigateTo`, etc.
- **Components**: All `.vue` files in `app/components/`
- **Composables**: All `use*.ts` files in `app/composables/`
- **Drizzle ORM (server)**: `db` instance from `~/server/db/index.ts`
- **Drizzle tables (server)**: Tables from `~/server/db/schema`

## Async Functions

Functions that perform I/O operations should be async:

```typescript
export async function getProjectRoot(startDir?: string): Promise<string> {
  const dir = startDir || dirname(fileURLToPath(import.meta.url));
  // Async operations
}
```

### Async/Await Usage

Always use `async/await` for async operations, avoid `.then()` chains:

```typescript
// Good
const root = await getProjectRoot();
const data = await fs.readFile(filepath, 'utf-8');

// Avoid
getProjectRoot().then(root => {
  fs.readFile(filepath, 'utf-8').then(data => {
    // Nested callbacks
  });
});
```

## Error Handling

### Throwing Errors

Functions should throw descriptive errors with clear messages:

```typescript
export async function getProjectRoot(startDir?: string): Promise<string> {
  if (!packageJsonExists) {
    throw new Error("Could not find project root (no package.json found)");
  }
}
```

### Error Messages Guidelines

- Be specific about what went wrong
- Include context (what was expected vs what was found)
- Avoid technical jargon in user-facing errors

```typescript
// Good
throw new Error("Could not find project root (no package.json found)");

// Avoid
throw new Error("Root not found");
```

## Path Resolution

Use Node.js path utilities for cross-platform compatibility:

### Common Path Operations

```typescript
import { resolve, dirname, join, basename } from 'path';
import { fileURLToPath } from 'url';

// Resolve to absolute path
const absolutePath = resolve('/some/relative/path');

// Get directory name of a file path
const dir = dirname('/path/to/file.txt');

// Join path segments (platform-aware)
const filepath = join(root, 'subdir', 'file.txt');

// Get filename from path
const filename = basename('/path/to/file.txt'); // "file.txt"
```

### `import.meta.url` Usage

For ES modules, use `fileURLToPath` to convert `import.meta.url` to a file path:

```typescript
import { dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
```

## Function Exports

### Named Exports

Preferred for utilities and shared functions:

```typescript
export async function getProjectRoot(startDir?: string): Promise<string>
export function defineGenerator<T>(fn: () => Promise<T>): () => Promise<T>
export const PROJECT_ROOT = '/path/to/project';
```

### Default Exports

Used for generators and Vue components:

```typescript
// Generator
export default defineGenerator(async () => {
  // Logic
});

// Vue component
<script setup lang="ts">
// Component code
</script>
<template>
  <div>Template</div>
</template>
```

## Type Safety

### TypeScript Best Practices

- Use explicit return types for exported functions
- Leverage inferred types for internal functions
- Define interfaces for complex objects
- **Never use `as any`** — fix the root cause instead

```typescript
// Good - explicit return type for exported function
export async function getProjectRoot(startDir?: string): Promise<string> {
  return resolve(startDir || dirname(fileURLToPath(import.meta.url)));
}

// Acceptable - inferred type for internal function
function sanitizeInput(input: string) {
  return input.trim().toLowerCase();
}

// Interface for complex objects
export interface GeneratorOptions {
  output: string;
  format?: 'json' | 'yaml';
}
```

## Code Formatting

The project uses Prettier for code formatting with default configuration (no `.prettierrc` file).

### Prettier Integration

```typescript
import prettier from 'prettier';

const formattedCode = await prettier.format(code, {
  parser: "typescript",
});
```

### Formatting Guidelines

- Use 2-space indentation (Prettier default)
- Use single quotes for strings (Prettier default)
- Use semicolons (Prettier default)
- Trailing commas where valid (Prettier default)

## Database Query Patterns

### Server Database (PostgreSQL)

```typescript
import { db } from '#server/db';
import { stories } from '#server/db/schema';
import { eq, and, desc } from 'drizzle-orm';

// Simple select
const allStories = await db.select().from(stories);

// With where clause
const oneStory = await db.select().from(stories)
  .where(eq(stories.id, 1));

// With relations
const storyWithAuthor = await db.query.stories.findFirst({
  where: eq(stories.id, 1),
  with: {
    author: true,
  },
});
```

### Local Database (SQLite via PowerSync)

```typescript
import { useLocalDb } from '~/composables/useLocalDb';
import { localSessions, localPages } from '#shared/db/localSchema';
import { eq } from 'drizzle-orm';

const db = useLocalDb();

// Simple select
const sessions = await db.select().from(localSessions);

// With where clause
const pages = await db.select().from(localPages)
  .where(eq(localPages.sessionId, sessionId));

// Insert
await db.insert(localPages).values({
  id: crypto.randomUUID(),
  sessionId: sessionId,
  system: 'system prompt',
  prompt: 'user input',
  response: 'AI response',
  createdAt: new Date().toISOString(),
});
```

## LLM Streaming Pattern

All LLM streaming must go through `useLlmStream` — never parse SSE inline:

```typescript
import { streamLlmFull } from '~/composables/useLlmStream';
import { PERSONA_PLATFORM } from '#shared/prompts';

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

## Related Documentation

- [Project Structure](./project-structure.md) - File organization and directory layout
- [Database Schema](./database-schema.md) - Table definitions and query patterns
- [API Routes](./api-routes.md) - Endpoint documentation
- [Frontend Architecture](./frontend-architecture.md) - Pages, components, and styling conventions
