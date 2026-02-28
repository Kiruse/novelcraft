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

## Import Patterns

### ES Module Imports

All imports use ES module syntax:

```typescript
import { access, constants } from "fs/promises";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";
```

### Local Module Imports

Local module imports must use `.js` extension for ESM compatibility:

```typescript
// From same directory
import { getProjectRoot, defineGenerator } from './utils/index.js';

// From parent directories
import { defineGenerator } from '../../utils/index.js';

// Or use index-less imports (handled by TypeScript)
import { getProjectRoot } from '../../utils';
```

**Important**: Node.js built-in modules (e.g., `fs/promises`, `path`) do not require `.js` extensions.

### Auto-imports

The following are auto-imported by Nuxt and do not require explicit imports:

- **Vue composables**: `ref`, `computed`, `onMounted`, etc.
- **Nuxt utilities**: `useFetch`, `useRoute`, `navigateTo`, etc.
- **Drizzle ORM**: `db` instance (from `~/server/db/index.ts`)
- **Database queries**: Tables from `~/server/db/schema`

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

### Drizzle ORM Usage

```typescript
import { db } from '~/server/db';
import { stories } from '~/server/db/schema';
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
    gameSessions: true,
  },
});
```

## Related Documentation

- [Project Structure](./project-structure.md) - File organization and directory layout
- [Generator System](./generator-system.md) - CLI generator patterns and conventions
- [Database Schema](./database-schema.md) - Table definitions and query patterns
