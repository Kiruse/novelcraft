import { v } from 'convex/values';
import { internal } from './_generated/api';
import { internalQuery, mutation } from './_generated/server';
import { Id } from './_generated/dataModel';

export const _getByClerk = internalQuery({
  args: {
    clerkId: v.string(),
  },
  handler: async (ctx, args) => {
    return await ctx.db.query('users').withIndex('by_clerkId', (q) => q.eq('clerkId', args.clerkId)).first();
  },
});

export const upsert = mutation({
  args: {},
  handler: async (ctx, args): Promise<Id<'users'>> => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new Error('Unauthenticated');

    const user = await ctx.runQuery(internal.users._getByClerk, { clerkId: identity.subject });
    const username = identity.preferredUsername ?? identity.nickname ?? identity.givenName ?? identity.name ?? 'Anon';

    if (user) {
      await ctx.db.patch(user._id, { username });
      return user._id;
    } else {
      return await ctx.db.insert('users', { clerkId: identity.subject, username });
    }
  },
});
