import { sqliteTable, text, integer } from 'drizzle-orm/sqlite-core';

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
  createdAt: text('created_at').notNull(),
});

export const localModuleRuntime = sqliteTable('local_module_runtime', {
  id: text('id').primaryKey().notNull(),
  sessionId: text('session_id').notNull(),
  moduleId: text('module_id').notNull(),
  data: text('data').notNull(),
});

export const localProfiles = sqliteTable('local_profiles', {
  id: text('id').primaryKey().notNull(),
  name: text('name').notNull(),
  fields: text('fields').notNull(),
  active: integer('active', { mode: 'boolean' }).notNull().default(false),
  createdAt: text('created_at').notNull(),
  updatedAt: text('updated_at').notNull(),
});

export const drizzleSchema = {
  localSessions,
  localPages,
  localModuleRuntime,
  localProfiles,
};
