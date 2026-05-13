import { NPCModule } from './npcModule';
import { PlanModule } from './planModule';
import { LoreModule } from './loreModule';
import { GameplayModuleRegistry } from './gameplayModule';

export const createDefaultRegistry = () =>
  new GameplayModuleRegistry([
    NPCModule,
    PlanModule,
    LoreModule,
  ]);

export {
  defineGameplayModule,
  toolOk,
  toolErr,
  toolCallRecordSchema,
  GameplayModuleRegistry,
} from './gameplayModule';

export type {
  GameplayModule,
  GameplayModuleContext,
  GameplaySession,
  ToolDefinition,
  ToolResult,
  ToolCallRecord,
} from './gameplayModule';

export { NPCModule } from './npcModule';
export { PlanModule } from './planModule';
export { LoreModule } from './loreModule';
