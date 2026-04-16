import z from 'zod';
import { defineGameplayModule, toolErr, toolOk } from './gameplayModule';

const configV1 = z.object({
  version: z.literal(1),
  locations: z.array(z.object({
    /** Descriptive name of the location, e.g. Castle Volkihar, Mura Village, Brimstone Cave, Grimwoods, etc */
    name: z.string(),
    /** Description of this place for advanced queries. */
    description: z.string(),
    /** Connections to other locations. Key is the place name, value is the distance in meters.
     * Likely to change in future versions.
     */
    connections: z.record(z.string(), z.number()),
  })),
});

const stateV1 = z.object({
  version: z.literal(1),
  /** Current location of the player. */
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
      // If current location found, assert validity of destination
      // Otherwise, just accept any destination to get back in the known world
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
