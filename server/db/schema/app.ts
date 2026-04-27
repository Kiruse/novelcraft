import { relations } from "drizzle-orm";
import { pgTable, serial, text, integer, timestamp, jsonb, index, pgEnum, uniqueIndex, boolean } from "drizzle-orm/pg-core";
import { user } from "./auth";
import { createInsertSchema } from "drizzle-zod";
import { z } from "zod";

export const messageRoleEnum = pgEnum("message_role", ["system", "agent", "user", "toolcall"]);

// ─── Table definitions (must come before relations) ───

export const stories = pgTable(
  "stories",
  {
    id: serial("id").primaryKey(),
    storyId: text("story_id").notNull(),
    authorId: text("author_id")
      .notNull()
      .references(() => user.id, { onDelete: "restrict" }),
    version: integer("version").notNull(),
    title: text("title").notNull(),
    description: text("description"),
    coverArt: text("cover_art"),
    genre: text("genre"),
    isVignette: boolean("is_vignette").notNull().default(false),
    modules: jsonb("modules").notNull().$type<Record<string, unknown>>(),
    createdAt: timestamp("created_at").defaultNow().notNull(),
    updatedAt: timestamp("updated_at")
      .defaultNow()
      .$onUpdate(() => new Date())
      .notNull(),
  },
  (table) => [uniqueIndex("stories_author_story_version_idx").on(table.authorId, table.storyId, table.version)],
);

export const gameSessions = pgTable(
  "game_sessions",
  {
    id: serial("id").primaryKey(),
    playerId: text("player_id")
      .notNull()
      .references(() => user.id, { onDelete: "cascade" }),
    storyId: integer("story_id")
      .notNull()
      .references(() => stories.id, { onDelete: "cascade" }),
    data: jsonb("data").notNull().$type<Record<string, unknown>>(),
    createdAt: timestamp("created_at").defaultNow().notNull(),
    updatedAt: timestamp("updated_at")
      .defaultNow()
      .$onUpdate(() => new Date())
      .notNull(),
  },
  (table) => [index("game_sessions_player_updated_idx").on(table.playerId, table.updatedAt)]
);

export const moduleRuntime = pgTable(
  "module_runtime",
  {
    id: serial("id").primaryKey(),
    gameSessionId: integer("game_session_id")
      .notNull()
      .references(() => gameSessions.id, { onDelete: "cascade" }),
    moduleId: text("module_id").notNull(),
    data: jsonb("data").notNull().$type<Record<string, unknown>>(),
  },
  (table) => [index("module_runtime_session_idx").on(table.gameSessionId)]
);

// TODO: This was used in an older version of the app and is only here in case
// this experiment fails and we have to revert back to messages. Remove once locked in.
export const gameSessionMessages = pgTable(
  "game_session_messages",
  {
    id: serial("id").primaryKey(),
    gameSessionId: integer("game_session_id")
      .notNull()
      .references(() => gameSessions.id, { onDelete: "cascade" }),
    role: messageRoleEnum("role").notNull(),
    contents: text("contents").notNull(),
    toolcallData: jsonb("toolcall_data").$type<unknown>(),
    createdAt: timestamp("created_at").defaultNow().notNull(),
  },
  (table) => [index("game_session_messages_session_created_idx").on(table.gameSessionId, table.createdAt)]
);

export const gameSessionPages = pgTable(
  "game_session_pages",
  {
    id: serial("id").primaryKey(),
    gameSessionId: integer("game_session_id")
      .notNull()
      .references(() => gameSessions.id, { onDelete: "cascade" }),
    system: text("system"),
    prompt: text("prompt"),
    response: text("response"),
    createdAt: timestamp("created_at").defaultNow().notNull(),
  },
  (table) => [index("game_session_pages_session_idx").on(table.gameSessionId, table.createdAt)]
);

// ─── Relations ───

export const storyRelations = relations(stories, ({ one, many }) => ({
  author: one(user, {
    fields: [stories.authorId],
    references: [user.id],
  }),
  gameSessions: many(gameSessions),
}));

export const gameSessionRelations = relations(gameSessions, ({ one, many }) => ({
  player: one(user, {
    fields: [gameSessions.playerId],
    references: [user.id],
  }),
  story: one(stories, {
    fields: [gameSessions.storyId],
    references: [stories.id],
  }),
  moduleRuntime: many(moduleRuntime),
  messages: many(gameSessionMessages),
  pages: many(gameSessionPages),
}));

export const moduleRuntimeRelations = relations(moduleRuntime, ({ one }) => ({
  gameSession: one(gameSessions, {
    fields: [moduleRuntime.gameSessionId],
    references: [gameSessions.id],
  }),
}));

export const gameSessionPageRelations = relations(gameSessionPages, ({ one }) => ({
  gameSession: one(gameSessions, {
    fields: [gameSessionPages.gameSessionId],
    references: [gameSessions.id],
  }),
}));

export const gameSessionMessageRelations = relations(gameSessionMessages, ({ one }) => ({
  gameSession: one(gameSessions, {
    fields: [gameSessionMessages.gameSessionId],
    references: [gameSessions.id],
  }),
}));

// ─── Validation schemas ───

export const storyModuleVal = z.record(z.string(), z.unknown());

export const insertStorySchema = createInsertSchema(stories);
export const insertGameSessionSchema = createInsertSchema(gameSessions);
export const insertModuleRuntimeSchema = createInsertSchema(moduleRuntime);
export const insertGameSessionMessageSchema = createInsertSchema(gameSessionMessages);
export const insertGameSessionPageSchema = createInsertSchema(gameSessionPages);
