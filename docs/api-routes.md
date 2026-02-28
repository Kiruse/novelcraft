# API Routes

This document describes the API routing system, endpoint conventions, available endpoints, and how to create new routes.

## Overview

The application uses Nuxt's file-based routing for API endpoints. API routes are located in `server/api/` and are automatically mapped to HTTP endpoints based on the file path and naming convention.

### Endpoint Conventions

#### File Naming Pattern

API route files follow the pattern `[resource].[method].ts`:

- **`[resource]`**: The resource name (e.g., `stories`, `sessions`)
- **`[method]`**: The HTTP method in lowercase (e.g., `get`, `post`, `put`, `delete`, `patch`)

#### Examples

| File Path | HTTP Endpoint | Method |
|-----------|---------------|--------|
| `server/api/stories.get.ts` | `/api/stories` | GET |
| `server/api/stories.post.ts` | `/api/stories` | POST |
| `server/api/stories/[id].get.ts` | `/api/stories/:id` | GET |
| `server/api/sessions.get.ts` | `/api/sessions` | GET |

#### Dynamic Routes

Use brackets `[param]` for URL parameters:

```typescript
// server/api/stories/[id].get.ts
export default defineEventHandler(async (event) => {
  const id = getRouterParam(event, "id");
  // Use id parameter
});
```

## Auto-Imports

The following are auto-imported in API route files:

- **`db`**: Database instance from `~/server/db/index.ts`
- **`defineEventHandler`**: Nuxt event handler function
- **Database queries**: Tables from `~/server/db/schema` (e.g., `stories`, `gameSessions`)

## Available Endpoints

### GET /api/stories

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

**Example Response:**

```json
{
  "stories": [
    {
      "id": 1,
      "storyId": "550e8400-e29b-41d4-a716-446655440000",
      "authorId": "user-123",
      "version": 1,
      "title": "The Dragon's Quest",
      "description": "An epic adventure",
      "coverArt": "https://example.com/cover.jpg",
      "genre": "fantasy",
      "createdAt": "2024-01-15T10:30:00.000Z",
      "updatedAt": "2024-01-15T10:30:00.000Z",
      "author": {
        "id": "user-123",
        "name": "John Doe"
      }
    }
  ]
}
```

### GET /api/sessions

Fetches the current user's recent game sessions (max 5) ordered by last update. Returns empty array if not authenticated.

**File:** `server/api/sessions.get.ts`

**Authentication:** Required (Better-Auth session)

**Query Parameters:** None

**Response:**

```typescript
{
  sessions: Array<{
    id: number;
    playerId: string;
    storyId: number;
    data: Record<string, unknown>;
    createdAt: Date;
    updatedAt: Date;
    story: {
      id: number;
      title: string;
      coverArt: string | null;
      genre: string | null;
    };
  }>
}
```

**Example Response:**

```json
{
  "sessions": [
    {
      "id": 1,
      "playerId": "user-123",
      "storyId": 1,
      "data": { "progress": 50, "inventory": [] },
      "createdAt": "2024-01-15T10:30:00.000Z",
      "updatedAt": "2024-01-16T14:20:00.000Z",
      "story": {
        "id": 1,
        "title": "The Dragon's Quest",
        "coverArt": "https://example.com/cover.jpg",
        "genre": "fantasy"
      }
    }
  ]
}
```

### GET /api/stories/:id

Fetches story details by ID.

**File:** `server/api/stories/[id].get.ts`

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Story ID |

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

**Error Responses:**

```typescript
// 400 Bad Request
{ "error": "Story ID is required" }

// 404 Not Found
{ "error": "Story not found" }
```

**Example Response:**

```json
{
  "story": {
    "id": 1,
    "storyId": "550e8400-e29b-41d4-a716-446655440000",
    "authorId": "user-123",
    "version": 1,
    "title": "The Dragon's Quest",
    "description": "An epic adventure",
    "coverArt": "https://example.com/cover.jpg",
    "genre": "fantasy",
    "modules": { "modules": [] },
    "createdAt": "2024-01-15T10:30:00.000Z",
    "updatedAt": "2024-01-15T10:30:00.000Z",
    "author": {
      "id": "user-123",
      "name": "John Doe",
      "image": "https://example.com/avatar.jpg"
    }
  }
}
```

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
// server/api/stories/[id].put.ts
import { db } from "../db";
import { stories } from "../db/schema";
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
    .update(stories)
    .set(body)
    .where(eq(stories.id, Number(id)))
    .returning();

  if (!result.length) {
    throw createError({
      statusCode: 404,
      statusMessage: "Story not found",
    });
  }

  return result[0];
});
```

### DELETE Endpoint

```typescript
// server/api/stories/[id].delete.ts
import { db } from "../db";
import { stories } from "../db/schema";
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
    .delete(stories)
    .where(eq(stories.id, Number(id)))
    .returning();

  if (!result.length) {
    throw createError({
      statusCode: 404,
      statusMessage: "Story not found",
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
// server/api/sessions.get.ts
export default defineEventHandler(async (event) => {
  const session = await getSession(event);

  if (!session) {
    return { sessions: [] };
  }

  const sessions = await db.query.gameSessions.findMany({
    where: eq(gameSessions.playerId, session.user.id),
    orderBy: desc(gameSessions.updatedAt),
    limit: 5,
  });

  return { sessions };
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

### Query with Multiple Relations

```typescript
export default defineEventHandler(async (event) => {
  const story = await db.query.stories.findFirst({
    where: eq(stories.id, 1),
    with: {
      author: true,
      gameSessions: {
        with: {
          player: true,
          messages: true,
        },
      },
    },
  });

  return { story };
});
```

### Nested Relations Filtering

```typescript
export default defineEventHandler(async (event) => {
  const story = await db.query.stories.findFirst({
    where: eq(stories.id, 1),
    with: {
      gameSessions: {
        orderBy: desc(gameSessions.updatedAt),
        limit: 5,
      },
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
