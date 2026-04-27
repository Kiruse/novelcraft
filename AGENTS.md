# Agents Documentation

High-level overview for AI agents working on the NovelCraft project. For comprehensive documentation, see the [detailed docs](./docs/).

**IMPORTANT:** Whenever you make changes, assert validity by running `bun run typecheck`, then summarize & document the changes with the `@docs-writer` subagent.

## Quick Reference

### Project Structure

```
novelcraft/
├── app/                    # Frontend (Nuxt 4 + Vue 3)
│   ├── pages/              # File-based routing
│   ├── components/         # Auto-imported Vue components
│   └── assets/css/         # Global styles
├── scripts/                # Development scripts
│   └── utils/              # Shared utilities for scripts
├── server/
│   ├── api/                # API routes (file-based)
│   ├── db/
│   │   ├── schema/         # Drizzle schema definitions
│   │   └── migrations/     # Database migrations
│   └── auth.ts             # Better-Auth config
├── docs/                   # Comprehensive documentation
├── nuxt.config.ts          # Nuxt configuration
└── package.json            # Dependencies & scripts
```

**Detailed docs:** [Project Structure](./docs/project-structure.md)

---

## Key Conventions Summary

### File Organization

| Category | Location | Pattern |
|----------|----------|---------|
| Generators | `scripts/generators/` | Default export, `defineGenerator()` wrapper |
| Utilities | `scripts/utils/` | Named exports, centralized via `index.ts` |
| API Routes | `server/api/` | `[resource].[method].ts` |
| Pages | `app/pages/` | `[route].vue`, `[param].vue` for dynamic |
| Components | `app/components/` | PascalCase `.vue`, auto-imported |
| Schema | `server/db/schema/` | Separate files: `auth.ts`, `app.ts`, `placeholder.ts` |

**Detailed docs:** [Code Conventions](./docs/code-conventions.md)

### Import Patterns

**Always use alias imports — never relative `../` paths.**

| Alias | Resolves to | Use in |
|-------|------------|----------|
| `~/` or `@/` | `app/` | App code (pages, components, composables) |
| `#server/` | `server/` | Server code (API routes, plugins, gameplay) |
| `#shared/` | `shared/` | Shared code (from both app and server) |
| `~~/` | Project root | Escape hatch (avoid unless necessary) |

- **Node.js built-ins**: No extension required (`fs/promises`, `path`)
- **Auto-imports**: Components, composables, `db` instance, Drizzle tables

### AI / Model Configuration

**Never use `gateway` from the `ai` package.** All models are resolved through `#server/ai/models` → `resolveModel(name)`.

- Models are defined in `server/ai/models.ts` as a hard-coded `Record<string, ModelConfig>`
- Each entry maps a model ID → `{ baseURL, apiKey? }`
- Always uses `@ai-sdk/openai-compatible` (`createOpenAICompatible`) — no provider-specific SDKs
- To add a new model: add an entry to the `models` map, then call `resolveModel('your-model-id')`
- To use a model: `import { resolveModel } from '#server/ai/models'; const model = resolveModel('meta-llama/llama-3.3-70b-instruct');`

### Agent / LLM Integration — Separation of Concerns

**All agent/LLM calls must be isolated from endpoint logic.**

- **LLM calls** go through `POST /api/llm/prompt` — a single, generic streaming endpoint that takes `{ model, messages[], persona }` and returns SSE events
- **CRUD endpoints** (vignettes, pages, sessions) handle only data persistence — no LLM calls
- **Frontend orchestrates**: calls the CRUD endpoint to create/prepare data, then calls `/api/llm/prompt` to generate text, then calls the CRUD endpoint again to persist the result

This separation enables:
- **Bring-your-own-agent**: swap the LLM endpoint for a third-party server without touching CRUD logic
- **Premium agent servers**: route requests to different backends based on plan/feature
- **Testability**: mock the LLM endpoint independently of data logic

**Prompts and personas** are defined in `shared/prompts.ts` so both frontend and server can access them.
Always import from `#shared/prompts` & maintain them there as a single source of truth.

**Important terminology:** A "persona" is ONLY the system prompt passed as the `persona` parameter to the LLM call — it defines who the agent *is*. The sole persona used throughout the app is `PERSONA_PLATFORM`. Everything else — scene instructions (`SYSTEM_VIGNETTE_OPEN`), steering notes (`SYSTEM_STEER`), editor requests (`SYSTEM_INSTRUCT`), page-level `system` fields — are **NOT** personas. They are regular messages with `author: 'system'` injected into the conversation history to guide the agent's behavior.

### TypeScript — No `as any`

**Never use `as any` casts.** If a type incompatibility arises, fix the root cause:

- Widen or narrow the source/target types (e.g. `Record<string, unknown>` instead of `unknown`)
- Fix schema types (e.g. Drizzle `.$type<>()`)
- Remove unnecessary type wrappers (e.g. `DeepReadonly` that creates cascading `Readonly<unknown>` constraints)
- Use targeted type assertions like `as ExpectedType` when crossing package boundaries
- If a genuine cross-package type mismatch exists (e.g. aikit vs ai SDK versions), **inform the user** rather than silently casting to `any`

### Module Type

ESM modules (`"type": "module"` in package.json)

### Styling — Open Props

The project uses [Open Props](https://open-props.style/) as its styling foundation. It provides CSS custom property design tokens for spacing, color, typography, shadows, radii, animations, and more.

- **Global styles**: `app/assets/css/app.css` — imports `open-props` (tokens) and `normalize` (reset)
- **Usage**: Reference tokens via `var(--size-3)`, `var(--gray-7)`, `var(--radius-2)`, `var(--shadow-2)`, `var(--font-size-4)`, etc.
- **Semantic tokens**: `--text-1`/`--text-2`, `--surface-1`..`--surface-4` for colors that adapt to dark mode
- **Brand**: `--brand-gradient` defined in `app.css` for the app's gradient
- **Rules**:
  - Never hardcode colors, spacing, radii, shadows, or font sizes — use Open Props tokens
  - Use logical properties (`inline-size`/`block-size`, `margin-block-start`, etc.)
  - Write scoped `<style scoped>` in Vue components
  - No class-name frameworks (Tailwind, etc.)

---

## Common Tasks

### Running the Application

```bash
# Development server
bun run dev

# Build for production
bun run build
```

### Database Operations

```bash
# Generate migrations from schema changes
bun run db:generate

# Apply migrations
bun run db:migrate

# Push schema directly (development)
bun run db:push

# Open Drizzle Studio
bun run db:studio
```

---

## Database Query Patterns

**Note** that the `#server` import alias is only reliably available from code in the `./server` folder. In `./shared`, use `~~/server` instead. In `./app`, neither `#server` nor `~~/server` should ever be used as this WILL BREAK THE APP.

### Basic Queries

```typescript
import { db } from '#server/db';
import { stories } from '#server/db/schema';
import { eq, desc } from 'drizzle-orm';

// Select all
const allStories = await db.select().from(stories);

// With where clause
const oneStory = await db.select().from(stories).where(eq(stories.id, 1));

// With relations
const storyWithAuthor = await db.query.stories.findFirst({
  where: eq(stories.id, 1),
  with: {
    author: true,
    gameSessions: true,
  },
});
```

### Runtime Validation

```typescript
import { insertStorySchema } from '#server/db/schema/app';

const validated = insertStorySchema.parse(data);
await db.insert(stories).values(validated);
```

**Detailed docs:** [Database Schema](./docs/database-schema.md)

---

## API Route Patterns

### Creating a New Endpoint

```typescript
// server/api/my-resource.get.ts
import { db } from "../db";
import { myTable } from "../db/schema";

export default defineEventHandler(async (event) => {
  const data = await db.select().from(myTable);
  return { data };
});
```

### Dynamic Routes

```typescript
// server/api/my-resource/[id].get.ts
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, "id");
  const data = await db.select().from(myTable).where(eq(myTable.id, id));
  return { data: data[0] };
});
```

### Available Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/stories` | GET | List all stories (with author) |
| `/api/stories/:id` | GET | Get story by ID (with author, modules) |
| `/api/sessions` | GET | Get current user's recent sessions |

**Detailed docs:** [API Routes](./docs/api-routes.md)

---

## Frontend Patterns

### Data Fetching

```typescript
const { data: stories } = await useFetch('/api/stories');
const { data: sessions } = await useFetch('/api/sessions');
```

### Route Parameters

```typescript
const route = useRoute();
const id = route.params.id;
```

### Component Props

```typescript
interface Props {
  story: {
    id: number;
    title: string;
    // ...
  };
}

defineProps<Props>();
```

**Detailed docs:** [Frontend Architecture](./docs/frontend-architecture.md)

---

## Agent Guidelines

### When to Modify Schema

- New application features require data structures
- Existing features need additional fields
- Relations need to be added/modified

**Process:**
1. Update `server/db/schema/app.ts`
2. Run `bun run db:generate` to create migration
3. Run `bun run db:migrate` to apply changes

### When to Add an API Route

- Frontend needs data not currently exposed
- New endpoints for external integrations
- CRUD operations for new entities

**Process:**
1. Create `[resource].[method].ts` in `server/api/`
2. Use `defineEventHandler` with `db` auto-import
3. Use Drizzle ORM for queries
4. Validate input with `drizzle-zod` schemas

### When to Create a Component

- Reusable UI patterns across multiple pages
- Complex logic that should be isolated
- Shared functionality (cards, modals, forms)

**Process:**
1. Create `.vue` file in `app/components/`
2. Use `defineProps<T>()` for TypeScript props
3. Use scoped styles to avoid conflicts
4. Component is auto-imported (no imports needed)

---

## Package Scripts

| Script | Purpose |
|--------|---------|
| `dev` | Start development server |
| `build` | Build for production |
| `generate` | Generate static site |
| `preview` | Preview production build |
| `db:generate` | Generate Drizzle migrations |
| `db:migrate` | Apply database migrations |
| `db:push` | Push schema to database (dev) |
| `db:studio` | Open Drizzle Studio |

---

## Detailed Documentation Links

| Document | Description |
|----------|-------------|
| [Project Structure](./docs/project-structure.md) | Complete file organization and directory layout |
| [Code Conventions](./docs/code-conventions.md) | Code styling, imports, patterns, and best practices |
| [Database Schema](./docs/database-schema.md) | Table definitions, relations, and validation |
| [API Routes](./docs/api-routes.md) | Endpoint documentation and route creation patterns |
| [Frontend Architecture](./docs/frontend-architecture.md) | Pages, components, and styling conventions |
