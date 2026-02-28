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

- **Node.js built-ins**: No extension required (`fs/promises`, `path`)
- **Local modules**: Use `.js` extension for ESM (`'./utils/index.js'`)
- **Auto-imports**: Components, composables, `db` instance, Drizzle tables

### Module Type

ESM modules (`"type": "module"` in package.json)

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
