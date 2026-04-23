import z from 'zod';
import { defineGameplayModule } from './gameplayModule';

const configV1 = z.object({
  version: z.literal(1),
  prompt: z.string(),
});

const stateV1 = z.object({
  version: z.literal(1),
});

export const SystemPromptModule = defineGameplayModule({
  type: 'system_prompt',
  config: configV1,
  state: stateV1,

  getKnowledge: ({ config }) => {
    return {
      systemPrompt: config.prompt,
    };
  },
});
