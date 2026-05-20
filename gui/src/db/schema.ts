import { sqliteTable, text, integer } from 'drizzle-orm/sqlite-core';

export const localStories = sqliteTable('local_stories', {
  id: text('id').primaryKey().notNull(),
  title: text('title').notNull(),
  description: text('description'),
  config: text('config').notNull(),
  createdAt: text('created_at').notNull(),
  updatedAt: text('updated_at').notNull(),
});

export const localSessions = sqliteTable('local_sessions', {
  id: text('id').primaryKey().notNull(),
  storyId: text('story_id').notNull(),
  title: text('title').notNull(),
  description: text('description'),
  createdAt: text('created_at').notNull(),
  updatedAt: text('updated_at').notNull(),
});

export const localPages = sqliteTable('local_pages', {
  id: text('id').primaryKey().notNull(),
  sessionId: text('session_id').notNull(),
  system: text('system'),
  prompt: text('prompt'),
  response: text('response'),
  toolCalls: text('tool_calls'),
  createdAt: text('created_at').notNull(),
});

export const localStateSnapshots = sqliteTable('local_state_snapshots', {
  id: text('id').primaryKey().notNull(),
  sessionId: text('session_id').notNull(),
  pageIndex: integer('page_index').notNull(),
  data: text('data').notNull(),
  createdAt: text('created_at').notNull(),
});

export const localLoreEntries = sqliteTable('local_lore_entries', {
  id: text('id').primaryKey().notNull(),
  storyId: text('story_id').notNull(),
  title: text('title').notNull(),
  content: text('content').notNull(),
  tags: text('tags'),
  createdAt: text('created_at').notNull(),
  updatedAt: text('updated_at').notNull(),
});

export const localProfiles = sqliteTable('local_profiles', {
  id: text('id').primaryKey().notNull(),
  name: text('name').notNull(),
  fields: text('fields').notNull(),
  active: integer('active').notNull().default(0),
  createdAt: text('created_at').notNull(),
  updatedAt: text('updated_at').notNull(),
});
