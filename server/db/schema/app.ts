import { relations } from "drizzle-orm";
import { pgTable, serial, text, integer, timestamp, jsonb, uniqueIndex } from "drizzle-orm/pg-core";
import { user } from "./auth";
import { createInsertSchema } from "drizzle-zod";
import { z } from "zod";

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
    modules: jsonb("modules").notNull().$type<Record<string, unknown>>(),
    createdAt: timestamp("created_at").defaultNow().notNull(),
    updatedAt: timestamp("updated_at")
      .defaultNow()
      .$onUpdate(() => new Date())
      .notNull(),
  },
  (table) => [uniqueIndex("stories_author_story_version_idx").on(table.authorId, table.storyId, table.version)],
);

// ─── Relations ───

export const storyRelations = relations(stories, ({ one }) => ({
  author: one(user, {
    fields: [stories.authorId],
    references: [user.id],
  }),
}));

// ─── Validation schemas ───

export const storyModuleVal = z.record(z.string(), z.unknown());

export const insertStorySchema = createInsertSchema(stories);
