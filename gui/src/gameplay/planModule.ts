import z from 'zod';
import { defineGameplayModule, toolOk } from './gameplayModule';

const stateV1 = z.object({
  version: z.literal(1),
  roadmap: z.string(),
});

const defaultState = { version: 1 as const, roadmap: '' };

export const PlanModule = defineGameplayModule({
  type: 'plan',
  state: stateV1,
  init: () => defaultState,

  getKnowledge: ({ state }) => {
    if (!state.roadmap) return {};
    return { roadmap: state.roadmap };
  },
})
  .withTool('updateRoadmap', {
    description: 'Update the story roadmap / todo list. Provide the full updated markdown content.',
    parameters: z.object({
      roadmap: z.string(),
    }),
    execute: ({ roadmap }, { state }) => {
      state.roadmap = roadmap;
      return toolOk();
    },
  });
