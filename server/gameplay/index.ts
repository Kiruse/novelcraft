import { registerModule } from './gameplayModule';
import { GraphMapModule } from './graphMapModule';
import { NPCModule } from './npcModule';
import { EventModule } from './eventModule';

/** Register all standard gameplay modules. Call once at app startup. */
export const registerStandardModules = () => {
  registerModule(GraphMapModule);
  registerModule(NPCModule);
  registerModule(EventModule);
};

export { registerModule, getModule, getAllModules, defineGameplayModule, findSessionModule, toolOk, toolErr } from './gameplayModule';
export type {
  GameplayModule,
  GameplayModuleContext,
  GameplaySession,
  GameplayModuleRuntime,
  GameplayModuleRuntimeDoc,
  ToolDefinition,
  ToolResult,
  Subagent,
} from './gameplayModule';

export { GraphMapModule } from './graphMapModule';
export { NPCModule } from './npcModule';
export { EventModule } from './eventModule';
