import z from 'zod';
import { defineGameplayModule, toolOk, toolErr } from './gameplayModule';

const rule = z.object({
  description: z.string(),
  script: z.string(),
});

const storyEvent = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string(),
  conditions: z.array(rule),
  triggers: z.array(rule),
});

const configV1 = z.object({
  version: z.literal(1),
  events: z.array(storyEvent),
  variables: z.array(z.object({
    name: z.string(),
    defaultValue: z.boolean(),
  })),
});

const stateV1 = z.object({
  version: z.literal(1),
  currentDay: z.number().int().min(1),
  currentTime: z.string().nullable(),
  variables: z.record(z.string(), z.boolean()),
  firedEvents: z.array(z.string()),
});

export const EventModule = defineGameplayModule({
  type: 'event',
  config: configV1,
  state: stateV1,

  getKnowledge: ({ config, state }) => {
    const unfired = config.events.filter((e) => !state.firedEvents.includes(e.id));

    const trueVariables = Object.entries(state.variables)
      .filter(([, v]) => v)
      .map(([name]) => name);

    return {
      events: {
        currentDay: state.currentDay,
        currentTime: state.currentTime,
        variables: trueVariables,
        eligibleEvents: unfired.map((e) => ({
          id: e.id,
          name: e.name,
          description: e.description,
          conditions: e.conditions.map((c) => c.description),
        })),
      },
    };
  },
})
  .withTool('event::advance-time', {
    description: 'Advance the story time by a number of days or set the time of day.',
    parameters: z.object({
      days: z.number().int().min(0).optional(),
      time: z.string().optional(),
    }),
    execute: ({ days, time }, { state }) => {
      const newState = { ...state };
      if (days && days > 0) {
        newState.currentDay += days;
        newState.firedEvents = [];
      }
      if (time !== undefined) {
        newState.currentTime = time;
      }
      return toolOk(newState);
    },
  })
  .withTool('event::set-variable', {
    description: 'Set a boolean event variable. This may activate or deactivate event conditions.',
    parameters: z.object({
      name: z.string(),
      value: z.boolean(),
    }),
    execute: ({ name, value }, { state }) => {
      return toolOk({
        ...state,
        variables: { ...state.variables, [name]: value },
      });
    },
  })
  .withTool('event::fire-event', {
    description: 'Mark an event as fired. Call this when the DM narrates the event occurring.',
    parameters: z.object({
      eventId: z.string(),
    }),
    execute: ({ eventId }, { state, config }) => {
      const event = config.events.find((e) => e.id === eventId);
      if (!event) return toolErr(`Event "${eventId}" not found`);

      let newState = { ...state };
      for (const trigger of event.triggers) {
        const setMatch = trigger.script.match(/set\s+(\w+)\s+to\s+(true|false)/i);
        if (setMatch?.[1] && setMatch[2]) {
          newState.variables = {
            ...newState.variables,
            [setMatch[1]]: setMatch[2].toLowerCase() === 'true',
          };
        }
      }

      return toolOk({
        ...newState,
        firedEvents: [...newState.firedEvents, eventId],
      });
    },
  });
