import z from 'zod';
import { defineGameplayModule, toolOk } from './gameplayModule';
import { commands } from '~/bindings';
import { unwrap } from '~/utils';

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
      if (!session.storyId) {
        return toolOk(undefined, {
          response: JSON.stringify([]),
        });
      }
      const entries = await unwrap(commands.loreQuery(session.storyId, query));

      return toolOk(undefined, {
        response: JSON.stringify(entries.map(e => ({
          id: e.id,
          title: e.title,
          content: e.content,
          tags: e.tags ?? [],
        }))),
      });
    },
  });
