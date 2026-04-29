# API Routes

This document describes the API routing system, endpoint conventions, available endpoints, and how to create new routes.

## Overview

The application uses Nuxt's file-based routing for API endpoints. API routes are located in `server/api/` and are automatically mapped to HTTP endpoints based on the file path and naming convention.

### Architecture

The server API is **pure CRUD** — it handles only auth, user data, and shareable story metadata. There are no gameplay, session, or vignette endpoints on the server. Gameplay state (vignettes, sessions, module runtime) is managed entirely client-side via PowerSync local SQLite.

The sole AI endpoint is `POST /api/llm/prompt`, a generic LLM proxy that streams SSE events.

### Endpoint Conventions

#### File Naming Pattern

API route files follow the pattern `[resource].[method].ts`:

- **`[resource]`**: The resource name (e.g., `stories`, `user`)
- **`[method]`**: The HTTP method in lowercase (e.g., `get`, `post`, `put`, `delete`, `patch`)

#### Examples

| File Path | HTTP Endpoint | Method |
|-----------|---------------|--------|
| `server/api/stories.get.ts` | `/api/stories` | GET |
| `server/api/stories.post.ts` | `/api/stories` | POST |
| `server/api/stories/[author]/[id].get.ts` | `/api/stories/:author/:id` | GET |
| `server/api/user/me.get.ts` | `/api/user/me` | GET |

#### Dynamic Routes

Use brackets `[param]` for URL parameters:

```typescript
// server/api/stories/[author]/[id].get.ts
export default defineEventHandler(async (event) => {
  const author = getRouterParam(event, "author");
  const id = getRouterParam(event, "id");
  // Use parameters
});
```

## Auto-Imports

The following are auto-imported in API route files:

- **`db`**: Database instance from `~/server/db/index.ts`
- **`defineEventHandler`**: Nuxt event handler function
- **Database queries**: Tables from `~/server/db/schema` (e.g., `stories`)

## Available Endpoints

### Story CRUD

#### GET /api/stories

Fetches all stories ordered alphabetically by title.

**File:** `server/api/stories.get.ts`

**Query Parameters:** None

**Response:**

```typescript
{
  stories: Array<{
    id: number;
    storyId: string;
    authorId: string;
    version: number;
    title: string;
    description: string | null;
    coverArt: string | null;
    genre: string | null;
    createdAt: Date;
    updatedAt: Date;
    author: {
      id: string;
      name: string;
    };
  }>
}
```

#### POST /api/stories

Creates a new story.

**File:** `server/api/stories.post.ts`

**Request Body:** Validated against `insertStorySchema`

**Response:** Created story object

#### GET /api/stories/[author]/[id]

Fetches story details by author and ID.

**File:** `server/api/stories/[author]/[id].get.ts`

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `author` | string | Yes | Author user ID |
| `id` | string | Yes | Story ID |

**Response:**

```typescript
{
  story: {
    id: number;
    storyId: string;
    authorId: string;
    version: number;
    title: string;
    description: string | null;
    coverArt: string | null;
    genre: string | null;
    modules: Record<string, unknown>;
    createdAt: Date;
    updatedAt: Date;
    author: {
      id: string;
      name: string;
      image: string | null;
    };
  }
}
```

#### PUT /api/stories/draft

Updates a story draft.

**File:** `server/api/stories/draft.put.ts`

#### POST /api/stories/publish

Publishes a new story version.

**File:** `server/api/stories/publish.post.ts`

### LLM Proxy

#### POST /api/llm/prompt

Streams an LLM response via Server-Sent Events (SSE). This is the only server-side AI endpoint.

**File:** `server/api/llm/prompt.post.ts`

**Request Body:**

```typescript
{
  model?: string;        // Model ID (resolved via resolveModel())
  persona: string;       // System persona (e.g., PERSONA_PLATFORM)
  messages: Array<{
    author: string;      // 'system' | 'user' | 'agent'
    content: string;
  }>;
}
```

**Response:** SSE stream with events:

| Event | Data | Description |
|-------|------|-------------|
| `text` | string | Text chunk from the LLM |
| `reasoning` | string | Reasoning/thinking chunk (if supported) |
| `done` | — | Stream completed |
| `error` | string | Error message |

**Frontend usage:** Always use `streamLlmFull()` or `streamLlm()` from `app/composables/useLlmStream.ts`. Never parse SSE inline.

### Auth & User

#### ALL /api/auth/[...all]

Better-Auth catch-all handler for authentication (login, signup, signout, etc.).

**File:** `server/api/auth/[...all].ts`

#### GET /api/user/me

Returns the currently authenticated user.

**File:** `server/api/user/me.get.ts`

**Authentication:** Required (Better-Auth session)

#### POST /api/user/redeem-author

Redeems an author role for the current user.

**File:** `server/api/user/redeem-author.post.ts`

**Authentication:** Required

#### GET /api/user/stories

Returns stories authored by the currently authenticated user.

**File:** `server/api/user/stories.get.ts`

**Authentication:** Required

## Creating New API Routes

### Simple List Endpoint

```typescript
// server/api/my-resource.get.ts
import { db } from "../db";
import { myTable } from "../db/schema";

export default defineEventHandler(async (event) => {
  const data = await db.select().from(myTable);
  return { data };
});
```

### Dynamic Route Parameter

```typescript
// server/api/my-resource/[id].get.ts
import { db } from "../db";
import { myTable } from "../db/schema";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, "id");

  if (!id) {
    throw createError({
      statusCode: 400,
      statusMessage: "ID is required",
    });
  }

  const data = await db.select().from(myTable).where(eq(myTable.id, Number(id)));

  if (!data.length) {
    throw createError({
      statusCode: 404,
      statusMessage: "Resource not found",
    });
  }

  return { data: data[0] };
});
```

### POST Endpoint with Body

```typescript
// server/api/stories.post.ts
import { db } from "../db";
import { stories } from "../db/schema";
import { insertStorySchema } from "../db/schema/app";

export default defineEventHandler(async (event) => {
  const body = await readBody(event);

  // Validate using drizzle-zod
  const validated = insertStorySchema.parse(body);

  const result = await db.insert(stories).values(validated).returning();
  return result[0];
});
```

### PUT/PATCH Endpoint

```typescript
// server/api/my-resource/[id].put.ts
import { db } from "../db";
import { myTable } from "../db/schema";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, "id");
  const body = await readBody(event);

  if (!id) {
    throw createError({
      statusCode: 400,
      statusMessage: "ID is required",
    });
  }

  const result = await db
    .update(myTable)
    .set(body)
    .where(eq(myTable.id, Number(id)))
    .returning();

  if (!result.length) {
    throw createError({
      statusCode: 404,
      statusMessage: "Resource not found",
    });
  }

  return result[0];
});
```

### DELETE Endpoint

```typescript
// server/api/my-resource/[id].delete.ts
import { db } from "../db";
import { myTable } from "../db/schema";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, "id");

  if (!id) {
    throw createError({
      statusCode: 400,
      statusMessage: "ID is required",
    });
  }

  const result = await db
    .delete(myTable)
    .where(eq(myTable.id, Number(id)))
    .returning();

  if (!result.length) {
    throw createError({
      statusCode: 404,
      statusMessage: "Resource not found",
    });
  }

  return { success: true };
});
```

## Query Parameters

### Reading Query Parameters

```typescript
// server/api/stories.get.ts
export default defineEventHandler(async (event) => {
  const query = getQuery(event);

  const limit = query.limit ? Number(query.limit) : 10;
  const offset = query.offset ? Number(query.offset) : 0;

  const data = await db.select()
    .from(stories)
    .limit(limit)
    .offset(offset);

  return { data };
});
```

## Authentication

### Accessing User Session

```typescript
export default defineEventHandler(async (event) => {
  const session = await getSession(event);

  if (!session) {
    throw createError({
      statusCode: 401,
      statusMessage: "Unauthorized",
    });
  }

  // Authenticated logic here
});
```

### Protected Routes

```typescript
export default defineEventHandler(async (event) => {
  const session = await getSession(event);

  if (!session) {
    throw createError({
      statusCode: 401,
      statusMessage: "Unauthorized",
    });
  }

  // Protected logic here
});
```

## Error Handling

### Using createError

```typescript
export default defineEventHandler(async (event) => {
  if (!param) {
    throw createError({
      statusCode: 400,
      statusMessage: "Parameter is required",
    });
  }
});
```

### Custom Error Responses

```typescript
export default defineEventHandler(async (event) => {
  try {
    const result = await someOperation();
    return result;
  } catch (error) {
    throw createError({
      statusCode: 500,
      statusMessage: error.message || "Internal server error",
    });
  }
});
```

## Database Queries with Relations

### Query with Single Relation

```typescript
export default defineEventHandler(async (event) => {
  const story = await db.query.stories.findFirst({
    where: eq(stories.id, 1),
    with: {
      author: true,
    },
  });

  return { story };
});
```

## Response Formatting

### Consistent Response Structure

```typescript
// Success response
return { stories: data };

// Error response (Nuxt handles this)
throw createError({
  statusCode: 404,
  statusMessage: "Resource not found",
});
```

### Date Serialization

Dates from Drizzle queries are automatically serialized to ISO strings by Nuxt.

## Related Documentation

- [Code Conventions](./code-conventions.md) - Database query patterns and async functions
- [Database Schema](./database-schema.md) - Table definitions and relation types
- [Project Structure](./project-structure.md) - File organization for API routes
- [Frontend Architecture](./frontend-architecture.md) - How the frontend consumes these endpoints
