import z from 'zod';
import { eq, desc, and, or, like } from 'drizzle-orm';
import { defineGameplayModule, toolOk } from './gameplayModule';
import { db, localLoreEntries } from '~/db';

const stateV1 = z.object({
  version: z.literal(1),
});

const defaultState = { version: 1 as const };

export const LoreModule = defineGameplayModule({
  type: 'lore',
  state: stateV1,
  init: () => defaultState,

  getKnowledge: () => {
    return {};
  },
})
  .withTool('query', {
    description: 'Search the lore database for in-world knowledge. Returns matching entries by title, content, or tags.',
    parameters: z.object({
      query: z.string(),
    }),
    execute: async ({ query }, { session }) => {
      const pattern = `%${query}%`;
      const entries = await db.select({
        id: localLoreEntries.id,
        title: localLoreEntries.title,
        content: localLoreEntries.content,
        tags: localLoreEntries.tags,
      }).from(localLoreEntries)
        .where(and(
          eq(localLoreEntries.storyId, session.storyId),
          or(
            like(localLoreEntries.title, pattern),
            like(localLoreEntries.content, pattern),
          ),
        ))
        .orderBy(desc(localLoreEntries.updatedAt))
        .limit(10);

      return toolOk(undefined, {
        response: JSON.stringify(entries.map(e => ({
          id: e.id,
          title: e.title,
          content: e.content,
          tags: e.tags ? JSON.parse(e.tags) : [],
        }))),
      });
    },
  });
