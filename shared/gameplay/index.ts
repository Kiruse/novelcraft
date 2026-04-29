import { registerModule } from './gameplayModule';
import { GraphMapModule } from './graphMapModule';
import { NPCModule } from './npcModule';
import { EventModule } from './eventModule';
import { SystemPromptModule } from './systemPromptModule';

export const registerStandardModules = () => {
  registerModule(SystemPromptModule);
  registerModule(EventModule);
  registerModule(NPCModule);
  registerModule(GraphMapModule);
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
export { SystemPromptModule } from './systemPromptModule';
