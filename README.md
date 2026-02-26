# NovelCraft

A Nuxt 4 application for collaborative fiction writing.

## Tech Stack

- **Runtime**: Bun
- **Framework**: Nuxt 4
- **Database**: Neon PostgreSQL
- **ORM**: Drizzle ORM with node-postgres driver
- **Authentication**: Better-Auth with Drizzle adapter
- **Validation**: drizzle-zod + Zod for runtime schema validation

## Setup

Make sure to install dependencies:

```bash
# npm
npm install

# pnpm
pnpm install

# yarn
yarn install

# bun
bun install
```

## Development Server

Start the development server on `http://localhost:3000`:

```bash
# npm
npm run dev

# pnpm
pnpm dev

# yarn
yarn dev

# bun
bun run dev
```

## Production

Build the application for production:

```bash
# npm
npm run build

# pnpm
pnpm build

# yarn
yarn build

# bun
bun run build
```

Locally preview production build:

```bash
# npm
npm run preview

# pnpm
pnpm preview

# yarn
yarn preview

# bun
bun run preview
```

Check out the [deployment documentation](https://nuxt.com/docs/getting-started/deployment) for more information.

## Database

NovelCraft uses Drizzle ORM with Bun's built-in SQL driver for database operations.

### Project Structure

```
server/
├── db/
│   ├── index.ts           # Database connection and db instance export
│   ├── schema/
│   │   ├── index.ts       # Schema export point
│   │   ├── auth.ts        # Better-Auth user tables
│   │   ├── app.ts         # App-specific tables (stories, sessions, etc.)
│   │   └── placeholder.ts # Placeholder tables
│   └── migrations/        # Generated migration files
```

### Configuration

- **Schema location**: `./server/db/schema/index.ts`
- **Migrations location**: `./server/db/migrations/`
- **Database driver**: `pg` (node-postgres) with `drizzle-orm/node-postgres`

The database connection reads `DATABASE_URL` from the `.env` file (Neon PostgreSQL).

### Available Scripts

```bash
# Generate migrations from schema changes
bun run db:generate

# Apply pending migrations to the database
bun run db:migrate

# Push schema directly to database (alternative to migrations)
bun run db:push

# Launch Drizzle Studio for database inspection
bun run db:studio
```

### Adding New Tables

1. Define your table in the appropriate schema file:

   - `server/db/schema/auth.ts` for Better-Auth tables
   - `server/db/schema/app.ts` for application tables
   - `server/db/schema/placeholder.ts` for temporary tables

```typescript
import { pgTable, serial, text, timestamp } from 'drizzle-orm/pg-core';

export const users = pgTable('users', {
  id: serial('id').primaryKey(),
  name: text('name').notNull(),
  email: text('email').notNull().unique(),
  createdAt: timestamp('created_at').notNull().defaultNow(),
});
```

2. Export from `server/db/schema/index.ts`:

```typescript
export * from './app';
export * from './auth';
export * from './placeholder';
```

3. Generate the migration:

```bash
bun run db:generate
```

4. Apply the migration:

```bash
bun run db:migrate
```

5. If using drizzle-zod for validation, the insert schema is automatically generated:

```typescript
import { createInsertSchema } from 'drizzle-zod';
import { users } from './schema/app';

export const insertUserSchema = createInsertSchema(users);
```

### Using the Database Instance

Import the `db` instance from `server/db`:

```typescript
import { db } from '~/server/db';
import { users } from '~/server/db/schema';
import { eq } from 'drizzle-orm';

// Query
const allUsers = await db.select().from(users);

// Filter
const user = await db.select().from(users).where(eq(users.id, 1));

// Insert
await db.insert(users).values({ name: 'John', email: 'john@example.com' });

// Update
await db.update(users).set({ name: 'Jane' }).where(eq(users.id, 1));

// Delete
await db.delete(users).where(eq(users.id, 1));
```

### Integration with Better-Auth

Better-Auth uses the Drizzle adapter for authentication:

```typescript
import { betterAuth } from 'better-auth';
import { drizzleAdapter } from 'better-auth/adapters/drizzle';
import { db } from './server/db';
```

The adapter is configured in `auth.ts` and automatically handles user sessions and authentication state.

### Application Schema Tables

The application schema is defined in `server/db/schema/app.ts` and includes four main tables:

**stories** - Stores story content and version tracking
- Links to users via `author_id` (text, FK → users.id, DELETE RESTRICT)
- Includes version tracking with `story_id` (UUID) and `version` (integer)
- Stores story configuration in `modules` (jsonb)

**game_sessions** - Stores player game sessions
- Links to users via `player_id` (text, FK → users.id, DELETE CASCADE)
- Links to stories via `story_id` (integer, FK → stories.id, DELETE CASCADE)
- Stores game state in `data` (jsonb)

**module_runtime** - Stores runtime state of modules
- Links to game_sessions via `game_session_id` (DELETE CASCADE)
- `module_id` is a text field (not a foreign key)
- Stores module-specific data in `data` (jsonb)

**game_session_messages** - Stores conversation history
- Links to game_sessions via `game_session_id` (DELETE CASCADE)
- Uses `message_role` enum for message roles
- Stores tool call data in `toolcall_data` (jsonb)

### Using Schema Relations

Drizzle relations enable type-safe queries with related data:

```typescript
import { db } from '~/server/db';
import { stories, gameSessions } from '~/server/db/schema';

// Query story with author and game sessions
const storyWithDetails = await db.query.stories.findFirst({
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
```

### Runtime Validation with drizzle-zod

The project uses `drizzle-zod` to generate Zod validation schemas from Drizzle table definitions:

```typescript
import {
  insertStorySchema,
  insertGameSessionSchema,
  insertModuleRuntimeSchema,
  insertGameSessionMessageSchema,
} from '~/server/db/schema/app';

// Validate data before insertion
const storyData = {
  storyId: 'uuid-123',
  authorId: 'user-uuid',
  version: 1,
  title: 'My Story',
  description: 'A description',
  modules: { modules: [] },
};

const validated = insertStorySchema.parse(storyData);
await db.insert(stories).values(validated);
```

**Extending with custom validation:**

```typescript
import { insertStorySchema } from '~/server/db/schema/app';
import { z } from 'zod';

const customStorySchema = insertStorySchema.extend({
  title: z.string().min(1).max(200),
  version: z.number().int().positive(),
});

const validated = customStorySchema.parse(data);
```

**API route validation example:**

```typescript
export default defineEventHandler(async (event) => {
  const body = await readBody(event);
  const validated = insertGameSessionSchema.parse(body);
  const session = await db.insert(gameSessions).values(validated).returning();
  return session[0];
});
```

## Scripts and Utilities

### Project Root Utility

The project includes a utility function to locate the project root directory:

```typescript
import { getProjectRoot } from '~/scripts/utils/index.js';

// Get absolute path to project root
const rootPath = await getProjectRoot();
// Or specify a starting directory
const rootPath = await getProjectRoot('/some/starting/path');
```

The utility traverses up the file system from a starting directory (or the module's location) until it finds a `package.json` file, which indicates the project root. This is useful for:

- Locating configuration files
- Reading project-relative paths
- Ensuring consistent path resolution across different execution contexts

If no `package.json` is found, the function throws an error.

## CLI Generator System

The project includes a CLI generator system built with Commander that automatically discovers and runs generator functions based on file hierarchy.

### Available Scripts

```bash
# Run a generator based on file path hierarchy
bun run cmd:generate <generator-path>
```

### Creating Generators

Generators are TypeScript files located in `scripts/generators/` that export a default async function. The CLI automatically discovers these files and creates subcommands based on their path.

**Example generator file structure:**

```
scripts/
└── generators/
    ├── mygen.ts              # Available as: bun run cmd:generate mygen
    └── auth/
        └── schema.ts          # Available as: bun run cmd:generate auth schema
```

**Creating a new generator:**

```typescript
// scripts/generators/mygen.ts
import { defineGenerator, getProjectRoot } from '~/scripts/utils/index.js';
import * as fs from 'fs/promises';
import * as path from 'path';

export default defineGenerator(async () => {
  const root = await getProjectRoot();
  const filepath = path.join(root, 'path/to/output.txt');
  await fs.writeFile(filepath, 'Generated content');
  console.log('Generator completed successfully');
});
```

### Example: Better Auth Schema Generator

The Better Auth schema generator is located at `scripts/generators/better-auth/schema.ts`:

```bash
# Generate the Better Auth Drizzle schema
bun run cmd:generate better-auth schema
```

This generator:
- Reads the Better Auth configuration from `server/auth/config`
- Generates Drizzle ORM schema definitions
- Outputs the formatted TypeScript code to `server/db/schema/auth-schema.ts`
