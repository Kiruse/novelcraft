import z from "zod";
import { defineGameplayModule, toolOk } from "./gameplayModule";

const configV1 = z.object({
  version: z.literal(1),
  npcs: z.array(z.object({
    name: z.string(),
    /** Initial location of this NPC - encoded depending on the map module. */
    initialLocation: z.string(),
    /** NPC personality prompt */
    personality: z.string().optional(),
    /** Natural disposition toward player characters, defaults to neutral */
    disposition: z.enum(['hostile', 'neutral', 'friendly']).optional(),
  })),
});

const stateV1 = z.object({
  version: z.literal(1),
  npcs: z.record(z.string(), z.object({
    /** Current location of the NPC. */
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
  // TODO: How does NPC moving work anyways?
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
