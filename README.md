# NovelCraft

A Nuxt 4 application for collaborative fiction writing.

## Tech Stack

- **Runtime**: Bun
- **Framework**: Nuxt 4 with Vue 3
- **Database**: Neon PostgreSQL
- **ORM**: Drizzle ORM with node-postgres driver
- **Authentication**: Better-Auth with Drizzle adapter
- **Validation**: drizzle-zod + Zod for runtime schema validation

## Setup

Install dependencies:

```bash
bun install
```

For local development, ensure you have [podman](https://podman.io/) installed — it's used to start local backend services such as Mailtrap.

## Development

Start the development server on `http://localhost:3000` and Mailtrap on `http://localhost:8025`:

```bash
bun dev
```

## Production

```bash
bun run build    # Build for production
bun run preview  # Preview production build
```

See the [Nuxt deployment docs](https://nuxt.com/docs/getting-started/deployment) for more information.

## Database

NovelCraft uses Drizzle ORM with Neon PostgreSQL.

```bash
bun run db:generate  # Generate migrations from schema changes
bun run db:migrate   # Apply pending migrations
bun run db:push      # Push schema directly (dev alternative)
bun run db:studio    # Launch Drizzle Studio for inspection
```

See [Database Schema](./docs/database-schema.md) for table definitions, relations, and query patterns.

## Documentation

| Document | Description |
|----------|-------------|
| [Project Structure](./docs/project-structure.md) | File organization and directory layout |
| [Code Conventions](./docs/code-conventions.md) | Styling, imports, patterns, and best practices |
| [Database Schema](./docs/database-schema.md) | Table definitions, relations, and validation |
| [API Routes](./docs/api-routes.md) | Endpoint documentation and route patterns |
| [Frontend Architecture](./docs/frontend-architecture.md) | Pages, components, and styling conventions |
