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
- Modules: `gameplayModule.ts` (core types + `GameplayModuleRegistry` class + `defineGameplayModule` factory with `.withTool()` builder), `npcModule.ts`, `planModule.ts`, `loreModule.ts`
- `createDefaultRegistry()` creates a `GameplayModuleRegistry` pre-loaded with `NPCModule`, `PlanModule`, `LoreModule` (uses constructor — no `register()` method)

```typescript
// gui/src/gameplay/index.ts
export const createDefaultRegistry = () =>
  new GameplayModuleRegistry([
    NPCModule,
    PlanModule,
    LoreModule,
  ]);

export {
  defineGameplayModule,
  toolOk,
  toolErr,
  toolCallRecordSchema,
  GameplayModuleRegistry,
} from './gameplayModule';

export type {
  GameplayModule,
  GameplayModuleContext,
  GameplaySession,
  ToolDefinition,
  ToolResult,
  ToolCallRecord,
} from './gameplayModule';

export { NPCModule } from './npcModule';
export { PlanModule } from './planModule';
export { LoreModule } from './loreModule';
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
import { createDefaultRegistry } from '~/gameplay';
```

### Auto-imports

A custom Vite plugin (`gui/vite-plugins/auto-import.ts`) automatically injects Vue composition API and vue-router imports into all `.ts` and `.vue` (`<script setup>`) files at build time. The plugin is registered in `gui/vite.config.ts` at `enforce: 'pre'` (before `@vitejs/plugin-vue`). Type declarations for these identifiers live in `gui/src/env.d.ts` (for TypeScript only — the Vite plugin handles runtime injection).

**Auto-imported identifiers (never import these manually — it causes duplicate import conflicts):**

- From `vue`: `ref`, `reactive`, `computed`, `watch`, `readonly`, `onMounted`, `onUnmounted`, `nextTick`
- From `vue-router`: `useRoute`, `useRouter`
- **Tauri**: `invoke`, `listen` (declared in `env.d.ts`, only used directly in `tauriLanguageModel.ts` for LLM streaming)

Components are **not** auto-imported — import explicitly:

```typescript
import StoryCard from '~/components/StoryCard.vue';
```

## Gameplay Module Patterns

### Module Definition

Modules are defined with `defineGameplayModule()` and augmented with `.withTool()`. Each module must define an `init()` method that returns the default state. Tools use immer draft mutation — mutate `ctx.state` directly and return `toolOk()` with no arguments (the draft changes are committed automatically via `finishDraft`). Only return `toolOk(newState)` when you need to override the entire state. Tool names in module definitions are unprefixed (e.g. `'move'`); `getToolSet()` auto-prefixes them to `${modType}::${toolDef.name}`.

```typescript
export const MyModule = defineGameplayModule({
  type: 'myModule',
  config: z.object({ /* config schema */ }),
  state: z.object({ /* state schema */ }),
  init: () => ({ version: 1, someField: '' }),
  getKnowledge(ctx) { return { key: ctx.state.someField }; },
}).withTool('doSomething', {
  description: 'Does something',
  parameters: z.object({ input: z.string() }),
  execute(params, ctx) {
    // Direct draft mutation — no spread needed
    ctx.state.someField = params.input;
    return toolOk();
  },
});
```

### Tool Result Helpers

```typescript
import { toolOk, toolErr } from '~/gameplay';

// Success — draft mutations apply (no state override)
return toolOk();

// Success — override entire state explicitly
return toolOk(newState);

// Success with response (for query-only tools that don't mutate state)
return toolOk(undefined, { response: 'Data returned to LLM' });

// Failure
return toolErr('Something went wrong');
```

### Immer State Transitions

Tool execution in `GameplayModuleRegistry.getToolSet()` and tool call replay in `useVignette.replay()` both delegate to `GameplayModuleRegistry.executeTool()`, which uses `immer` (`createDraft`/`finishDraft`) for immutable state updates. This means tools can mutate `ctx.state` directly without spread operators:

1. When module state is undefined, `module.init()` is called to produce default state (handled by `executeTool()`)
2. `createDraft(base)` creates a mutable draft proxy of the current module state
3. Draft is passed as `ctx.state` to the tool handler
3. Tool mutates `ctx.state` directly (e.g. `ctx.state.someField = value`)
4. If `toolOk()` is returned (no `state` argument), `finishDraft(draft)` produces the new immutable state
5. If `toolOk(newState)` is returned, the provided state overrides the draft entirely
6. If `toolErr(error)` is returned, `executeTool()` throws — the draft is discarded, no state change occurs

**Dependency:** `immer` ^11.1.6

### Building a Tool Set for LLM Calls

```typescript
import { createDefaultRegistry } from '~/gameplay';

const registry = createDefaultRegistry();
const toolSet = registry.getToolSet(session, (tool, params, moduleType, newState) => {
  // Track tool calls and state mutations during streaming
});
```

### Tool Call Records

Tool calls are persisted as JSON on pages via `toolCallRecordSchema`:

```typescript
import { toolCallRecordSchema, type ToolCallRecord } from '~/gameplay';

// Records stored in page.tool_calls as JSON array
const records: ToolCallRecord[] = [{ tool: 'npc::addNPC', params: { name: 'Alice' } }];
```

## Async Functions

Functions that perform I/O operations should be async:

```typescript
import { commands } from '~/bindings';
import { unwrap } from '~/utils';

export async function loadSessions() {
  return await unwrap(commands.sessionList());
}
```

### Async/Await Usage

Always use `async/await` for async operations, avoid `.then()` chains:

```typescript
// Good
const sessions = await unwrap(commands.sessionList());

// Avoid
unwrap(commands.sessionList()).then(sessions => {
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
export async function loadSessions() {
  return await unwrap(commands.sessionList());
}

// Interface for complex objects
export interface ProfileFields {
  name: string;
  appearance: string;
  interests: string;
  [key: string]: unknown;
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

// With gameplay module tools
import { createDefaultRegistry } from '~/gameplay';
const registry = createDefaultRegistry();
const tools = registry.getToolSet(session, onToolCall);

for await (const event of streamLlmFull({ persona: PERSONA_PLATFORM, messages, tools })) {
  if (event.type === 'text') {
    // Append text chunk
  }
  if (event.type === 'tool-call') {
    // Tool call: event.data = { id, tool, args } JSON
  }
  if (event.type === 'tool-result') {
    // Tool result: event.data = { id, tool, result } JSON
  }
  if (event.type === 'error') {
    // Handle error
  }
}
```

## Data Access Patterns

All data access goes through tauri-specta generated bindings (`commands.xxx()` from `~/bindings`), wrapped with `unwrap()` from `~/utils`. There is no database — data is stored as JSON files managed by the Rust backend.

```typescript
import { commands } from '~/bindings';
import { unwrap } from '~/utils';

// Sessions
const sessions = await unwrap(commands.sessionList());
const result = await unwrap(commands.sessionLoad(id));
await unwrap(commands.sessionSaveMeta(id, updatedMeta));
await unwrap(commands.sessionPushPage(id, pageEntry));
await unwrap(commands.sessionUpdatePage(id, 0, updatedPage));
await unwrap(commands.sessionTruncatePages(id, 5));
await unwrap(commands.sessionDelete(id));

// Snapshots
const head = await unwrap(commands.sessionGetHeadSnapshot(id));
await unwrap(commands.sessionSaveHeadSnapshot(id, snap));
await unwrap(commands.sessionDeleteHeadSnapshot(id));
const snap = await unwrap(commands.sessionFindSnapshotBefore(id, 5));
await unwrap(commands.sessionSaveCheckpoint(id, 0, snap));
await unwrap(commands.sessionDeleteCheckpointsFrom(id, 5));

// Profiles
const { profiles, active_id } = await unwrap(commands.profileList());
await unwrap(commands.profileCreate(newProfile));
await unwrap(commands.profileUpdate(profileId, updatedProfile));
await unwrap(commands.profileDelete(profileId));
await unwrap(commands.profileSetActive(profileId));

// Stories
const story = await unwrap(commands.storyGet(id));
await unwrap(commands.storySave(storyEntry));

// Lore
const results = await unwrap(commands.loreQuery(id, 'search term'));
```

**No transactions**: Each command call performs a single atomic file operation. The frontend drives sequential operations when multiple steps are needed.

```typescript
// Sequential operations — each call is independent
await unwrap(commands.sessionTruncatePages(id, 5));
await unwrap(commands.sessionDeleteHeadSnapshot(id));
await unwrap(commands.sessionDeleteCheckpointsFrom(id, 5));
```

## Rust Backend Formatting

The Rust backend (`engine/src/`) uses [`rustfmt`](https://rust-lang.github.io/rustfmt/) with a project-level config at `engine/src/rustfmt.toml`:

- **Indentation**: 2 spaces (`tab_spaces = 2`)
- Format with `just fmt`, verify with `just fmt-check`

## Related Documentation

- [Project Structure](./project-structure.md) - File organization and directory layout
- [Data Storage](./database-schema.md) - JSON file formats and storage layout
- [API Routes](./api-routes.md) - Tauri command documentation
- [Frontend Architecture](./frontend-architecture.md) - Pages, components, and styling conventions
