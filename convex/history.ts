import { v } from 'convex/values';
import { query, mutation, internalMutation } from './_generated/server';
import { internal } from './_generated/api';
import { paginationOptsValidator } from 'convex/server';
import { Id } from './_generated/dataModel';

export const create = mutation({
  args: {
    session: v.id('storySessions'),
    content: v.string(),
    userId: v.string(),
  },
  handler: async (ctx, args): Promise<Id<'history'> | undefined> => {
    // TODO: Implement proper authentication
    // For now, this is a placeholder that accepts any userId

    const isAuthorized = await ctx.runQuery(internal.storySessions._isAuthorized, {
      user: args.userId,
      session: args.session,
    });
    if (!isAuthorized) throw new Error('Unauthorized');

    const user = await ctx.runQuery(internal.users._getByUserId, { userId: args.userId });
    if (!user) throw new Error('User not found');

    const historyId = await ctx.db.insert('history', {
      session: args.session,
      author: user._id,
      content: args.content,
    });
    return historyId;
  },
});

export const _create = internalMutation({
  args: {
    session: v.id('storySessions'),
    author: v.string(),
    content: v.string(),
  },
  handler: async (ctx, args) => {
    return await ctx.db.insert('history', {
      session: args.session,
      author: args.author,
      content: args.content,
    });
  },
});

export const get = query({
  args: {
    id: v.id('history'),
    userId: v.string(),
  },
  handler: async (ctx, args) => {
    // TODO: Implement proper authentication
    // For now, this is a placeholder that accepts any userId

    const historyItem = await ctx.db.get(args.id);
    if (!historyItem) return null;

    const session = await ctx.db.get(historyItem.session);
    if (!session) throw new Error('Session not found');

    const isAuthorized = await ctx.runQuery(internal.storySessions._isAuthorized, {
      user: args.userId,
      session: historyItem.session,
    });
    if (!isAuthorized) throw new Error('Unauthorized');

    return historyItem;
  },
});

export const getLatestByAuthor = query({
  args: {
    session: v.id('storySessions'),
    author: v.string(),
    userId: v.string(),
  },
  handler: async (ctx, args) => {
    // TODO: Implement proper authentication
    // For now, this is a placeholder that accepts any userId

    const isAuthorized = await ctx.runQuery(internal.storySessions._isAuthorized, {
      user: args.userId,
      session: args.session,
    });
    if (!isAuthorized) throw new Error('Unauthorized');

    return await ctx.db
      .query('history')
      .withIndex('by_session_author', (q) =>
        q.eq('session', args.session).eq('author', args.author),
      )
      .order('desc')
      .first();
  },
});

export const list = query({
  args: {
    session: v.id('storySessions'),
    userId: v.string(),
    paginate: v.optional(paginationOptsValidator),
  },
  handler: async (ctx, args) => {
    // TODO: Implement proper authentication
    // For now, this is a placeholder that accepts any userId

    const isAuthorized = await ctx.runQuery(internal.storySessions._isAuthorized, {
      user: args.userId,
      session: args.session,
    });
    if (!isAuthorized) throw new Error('Unauthorized');

    return await ctx.db
      .query('history')
      .withIndex('by_session_author', (q) => q.eq('session', args.session))
      .paginate(args.paginate ?? { cursor: null, numItems: 100 });
  },
});
