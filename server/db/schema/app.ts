import { relations } from "drizzle-orm";
import { pgTable, serial, text, integer, timestamp, jsonb, index, pgEnum } from "drizzle-orm/pg-core";
import { user } from "./auth";
import { createInsertSchema } from "drizzle-zod";
import { z } from "zod";

export const messageRoleEnum = pgEnum("message_role", ["system", "agent", "user", "toolcall"]);

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
    modules: jsonb("modules").notNull().$type<unknown>(),
    createdAt: timestamp("created_at").defaultNow().notNull(),
    updatedAt: timestamp("updated_at")
      .defaultNow()
      .$onUpdate(() => new Date())
      .notNull(),
  },
  (table) => [index("stories_author_story_version_idx").on(table.authorId, table.storyId, table.version)]
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
    data: jsonb("data").notNull().$type<unknown>(),
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
    data: jsonb("data").notNull().$type<unknown>(),
  },
  (table) => [index("module_runtime_session_idx").on(table.gameSessionId)]
);

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
}));

export const moduleRuntimeRelations = relations(moduleRuntime, ({ one }) => ({
  gameSession: one(gameSessions, {
    fields: [moduleRuntime.gameSessionId],
    references: [gameSessions.id],
  }),
}));

export const gameSessionMessageRelations = relations(gameSessionMessages, ({ one }) => ({
  gameSession: one(gameSessions, {
    fields: [gameSessionMessages.gameSessionId],
    references: [gameSessions.id],
  }),
}));

// TODO: GameplayModule subsystem
const storyModuleSchema = z.record(z.string(), z.object({}));

const insertStorySchema = createInsertSchema(stories);

const insertGameSessionSchema = createInsertSchema(gameSessions);

const insertModuleRuntimeSchema = createInsertSchema(moduleRuntime);

const insertGameSessionMessageSchema = createInsertSchema(gameSessionMessages);

export { insertStorySchema, insertGameSessionSchema, insertModuleRuntimeSchema, insertGameSessionMessageSchema };
