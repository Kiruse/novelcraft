import z from 'zod';
import { defineGameplayModule, toolErr, toolOk } from './gameplayModule';

const configV1 = z.object({
  version: z.literal(1),
  locations: z.array(z.object({
    name: z.string(),
    description: z.string(),
    connections: z.record(z.string(), z.number()),
  })),
});

const stateV1 = z.object({
  version: z.literal(1),
  currentLocation: z.string(),
});

export const GraphMapModule = defineGameplayModule({
  type: 'map::graph',
  config: configV1,
  state: stateV1,

  getKnowledge: ({ config, state }) => {
    const currentLocation = config.locations.find(loc => loc.name === state.currentLocation);
    if (!currentLocation){
      return {
        location: {
          name: 'You are lost',
          connections: [config.locations[0]?.name ?? 'Nowhere'],
        },
      };
    }

    return {
      location: {
        name: state.currentLocation,
        connections: Object.entries(currentLocation.connections)
          .map(([name, distance]) => `${name} (${distance}m)`)
          .join(', '),
      },
    };
  },
})
  .withTool('nav::move', {
    description: 'Move to a new accessible location.',
    parameters: z.object({
      destination: z.string(),
    }),
    execute: ({ destination }, { config, state }) => {
      const currentLocation = config.locations.find(loc => loc.name === state.currentLocation);
      if (currentLocation) {
        if (!(destination in currentLocation.connections))
          return toolErr(`Destination ${destination} unreachable from ${currentLocation.name}`);
      }
      return toolOk({
        ...state,
        currentLocation: destination,
      });
    },
  });
