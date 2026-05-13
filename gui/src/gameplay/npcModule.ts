import z from "zod";
import { defineGameplayModule, toolOk } from "./gameplayModule";

const stateV1 = z.object({
  version: z.literal(1),
  npcs: z.record(z.string(), z.object({
    name: z.string(),
    location: z.string(),
    personality: z.string().optional(),
    disposition: z.enum(['hostile', 'neutral', 'friendly']).optional(),
  })),
});

const defaultState = { version: 1 as const, npcs: {} };

export const NPCModule = defineGameplayModule({
  type: 'npc',
  state: stateV1,
  init: () => defaultState,
  getKnowledge: ({ state }) => {
    return {
      npcs: Object.entries(state.npcs).map(([name, data]) => ({
        name,
        location: data.location,
      })),
    };
  },
})
  .withTool('move', {
    description: 'Move the NPC to a new location.',
    parameters: z.object({
      name: z.string(),
      destination: z.string(),
    }),
    execute: ({ name, destination }, { state }) => {
      if (!(name in state.npcs))
        return { success: false as const, error: `NPC "${name}" not found` };

      state.npcs[name]!.location = destination;
      return toolOk();
    },
  });
