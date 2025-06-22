import { defineSchema, defineTable } from 'convex/server';
import { v } from 'convex/values';

export const storyStatus = v.union(
  v.literal('private'),
  v.literal('unlisted'),
  v.literal('public'),
);

export default defineSchema({
  users: defineTable({
    clerkId: v.string(),
    username: v.string(),
  })
  .index('by_clerkId', ['clerkId']),

  stories: defineTable({
    name: v.string(),
    authors: v.array(v.string()),
    synopsis: v.string(),
    coverImage: v.string(),
    tags: v.array(v.string()),
    playCount: v.number(),
    status: storyStatus,
  })
  .index('by_name', ['name']),

  storySessions: defineTable({
    story: v.id('stories'),
    /** Clerk subject */
    primaryUser: v.string(),
    /** Clerk subjects that the primary user invited to the session. */
    secondaryUsers: v.array(v.string()),
    lastActiveAt: v.int64(),
    concludedAt: v.optional(v.int64()),
    result: v.optional(v.object({
      reason: v.string(),
      summary: v.string(),
    })),
    /** A list of commemorative NFTs minted for this session, if any. */
    memories: v.array(v.string()),
  })
  .index('by_primaryUser_story', ['primaryUser', 'story']),

  /** Conversation history of a story session. */
  history: defineTable({
    session: v.id('storySessions'),
    /** Author of the message. `system` for system messages, `ai` for AI messages, or the username.
     * Tool calls & their results are documented as `system` messages.
     */
    author: v.string(),
    content: v.string(),
  })
  .index('by_session_author', ['session', 'author']),
});
