import { v } from 'convex/values';
import { query, mutation, internalQuery } from './_generated/server';
import { DateTime } from 'luxon';
import { internal } from './_generated/api';
import { paginationOptsValidator } from 'convex/server';

/** Create a new story session. */
export const create = mutation({
  args: {
    story: v.id('stories'),
    secondaryUsers: v.optional(v.array(v.string())),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new Error('Unauthenticated');

    const sessionId = await ctx.db.insert('storySessions', {
      story: args.story,
      primaryUser: identity.subject,
      secondaryUsers: args.secondaryUsers ?? [],
      lastActiveAt: BigInt(DateTime.now().toUnixInteger()),
      memories: [],
    });
    return sessionId;
  },
});

/** Touch a story session to update the last active timestamp. */
export const touch = mutation({
  args: {
    id: v.id('storySessions'),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new Error('Unauthenticated');

    const isAuthorized = await ctx.runQuery(internal.storySessions._isAuthorized, {
      user: identity.subject,
      session: args.id,
    });
    if (!isAuthorized) throw new Error('Unauthorized');

    await ctx.db.patch(args.id, {
      lastActiveAt: BigInt(DateTime.now().toUnixInteger()),
    });
  },
});

/** Get a story session by ID. */
export const get = query({
  args: {
    id: v.id('storySessions'),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new Error('Unauthenticated');

    const session = await ctx.db.get(args.id);
    if (!session) {
      return null;
    }

    // must be primary or secondary user
    if (session.primaryUser !== identity.subject && !session.secondaryUsers.includes(identity.subject))
      throw new Error('Unauthorized');

    return session;
  },
});

/** List all story sessions of an authenticated user, optionally filtered by story. */
export const list = query({
  args: {
    story: v.optional(v.id('stories')),
    paginate: v.optional(paginationOptsValidator),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity)
      throw new Error('Unauthenticated');

    return await ctx.db
      .query('storySessions')
      .withIndex('by_primaryUser_story', (q) =>
        args.story
        ? q.eq('primaryUser', identity.subject).eq('story', args.story)
        : q.eq('primaryUser', identity.subject),
      )
      .paginate(args.paginate ?? { cursor: null, numItems: 100 });
  },
});

// Running this as internal query allows the convex backend to cache the results.
export const _isAuthorized = internalQuery({
  args: {
    user: v.string(),
    session: v.id('storySessions'),
  },
  handler: async (ctx, args) => {
    const session = await ctx.db.get(args.session);
    if (!session) return false;
    return session.primaryUser === args.user || session.secondaryUsers.includes(args.user);
  },
});

