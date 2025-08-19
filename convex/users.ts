import { v } from 'convex/values';
import { internal } from './_generated/api';
import { internalQuery, mutation } from './_generated/server';
import { Id } from './_generated/dataModel';

export const _getByUserId = internalQuery({
  args: {
    userId: v.string(),
  },
  handler: async (ctx, args) => {
    return await ctx.db.query('users').withIndex('by_userId', (q) => q.eq('userId', args.userId)).first();
  },
});

export const upsert = mutation({
  args: {
    userId: v.string(),
    username: v.string(),
  },
  handler: async (ctx, args): Promise<Id<'users'>> => {
    // TODO: Implement proper authentication
    // For now, this is a placeholder that accepts any userId and username

    const user = await ctx.runQuery(internal.users._getByUserId, { userId: args.userId });

    if (user) {
      await ctx.db.patch(user._id, { username: args.username });
      return user._id;
    } else {
      return await ctx.db.insert('users', { userId: args.userId, username: args.username });
    }
  },
});
