import z from "zod";
import { defineGameplayModule, toolOk } from "./gameplayModule";

const configV1 = z.object({
  version: z.literal(1),
  npcs: z.array(z.object({
    name: z.string(),
    initialLocation: z.string(),
    personality: z.string().optional(),
    disposition: z.enum(['hostile', 'neutral', 'friendly']).optional(),
  })),
});

const stateV1 = z.object({
  version: z.literal(1),
  npcs: z.record(z.string(), z.object({
    location: z.string(),
  })),
});

export const NPCModule = defineGameplayModule({
  type: 'npc',
  config: configV1,
  state: stateV1,
  getKnowledge: ({ config }) => {
    return {};
  },
})
  .withTool('npc::move', {
    description: 'Move the NPC to a connected destination location.',
    parameters: z.object({
      destination: z.string(),
    }),
    execute: ({ destination }, { state }) => {
      return toolOk({
        ...state,
      });
    },
  });
