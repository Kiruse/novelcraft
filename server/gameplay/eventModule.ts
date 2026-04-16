import z from 'zod';
import { defineGameplayModule, toolOk, toolErr } from './gameplayModule';

// --- Shared DSL rule schema (used by both conditions and triggers) ---

const rule = z.object({
  /** Natural language description of this rule. */
  description: z.string(),
  /** Domain-specific script (dummy for now, later parsed by Langium DSL). */
  script: z.string(),
});

// --- Event schema ---

const storyEvent = z.object({
  /** Unique identifier for this event. */
  id: z.string(),
  /** Human-readable name. */
  name: z.string(),
  /** Description of what happens when this event fires. */
  description: z.string(),
  /** Conditions under which this event can fire. All must be satisfied (AND). */
  conditions: z.array(rule),
  /** Triggers that execute when this event fires. */
  triggers: z.array(rule),
});

// --- Config & State ---

const configV1 = z.object({
  version: z.literal(1),
  events: z.array(storyEvent),
  /** Boolean variables that can be referenced by conditions and updated by triggers. */
  variables: z.array(z.object({
    name: z.string(),
    /** Default value. */
    defaultValue: z.boolean(),
  })),
});

const stateV1 = z.object({
  version: z.literal(1),
  /** Current story day (1-indexed). */
  currentDay: z.number().int().min(1),
  /** Current time within the day as "HH:MM", or null if not tracked. */
  currentTime: z.string().nullable(),
  /** Current values of all variables. */
  variables: z.record(z.string(), z.boolean()),
  /** IDs of events that have already fired (to prevent re-firing one-shot events). */
  firedEvents: z.array(z.string()),
});

export const EventModule = defineGameplayModule({
  type: 'event',
  config: configV1,
  state: stateV1,

  getKnowledge: ({ config, state }) => {
    // TODO: evaluate conditions via DSL interpreter
    // For now, report all unfired events as potentially eligible
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

      // Apply trigger effects (dummy: just log, later will execute DSL)
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
