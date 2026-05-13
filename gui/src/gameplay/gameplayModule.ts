import type { ToolSet } from 'ai';
import { zodSchema } from '@ai-sdk/provider-utils';
import { createDraft, finishDraft } from 'immer';
import z from 'zod';
import { ConversationalData } from '@stegakir/aikit/archetypes/conversational';
import { DeepReadonly } from 'vue';

type MaybePromise<T> = T | Promise<T>;

export interface GameplaySession {
  storyId: string;
  sessionId: string;
  /** Complete game session state. Modules lens into this object. */
  state: Record<string, unknown>;
}

export interface GameplayModule<
  T extends string = string,
  S extends z.ZodObject = z.ZodObject,
> {
  type: T;
  state: S;
  tools?: ToolDefinition<T, z.ZodObject, S>[];
  subagents?: Subagent[];
  init(): MaybePromise<z.infer<S>>;
  getKnowledge(ctx: DeepReadonly<GameplayModuleContext<T, S>>): object;
}

export interface ToolDefinition<T extends string, P extends z.ZodObject, S extends z.ZodObject> {
  name: string;
  description: string;
  parameters?: P;
  execute(params: z.infer<P>, context: GameplayModuleContext<T, S>): MaybePromise<ToolResult<z.infer<S>>>;
}

export interface GameplayModuleContext<T extends string, S extends z.ZodObject> {
  session: GameplaySession;
  module: GameplayModule<T, S>;
  state: z.infer<S>;
}

export interface Subagent {
  name: string;
  summary: string;
  prompt: ConversationalData;
}

export type ToolResult<S> =
  | { success: true; state?: S; response?: string }
  | { success: false; error: string };

export type OnToolCall = (tool: string, params: Record<string, unknown>, moduleType: string, newState: unknown) => void;

export type ToolCallRecord = z.infer<typeof toolCallRecordSchema>;
export const toolCallRecordSchema = z.object({
  tool: z.string(),
  params: z.record(z.string(), z.unknown()),
});

export class GameplayModuleRegistry {
  private _modules: Record<string, GameplayModule>;

  constructor(modules: GameplayModule[]) {
    this._modules = Object.fromEntries(modules.map(mod => [mod.type, mod]));
  }

  get = (type: string): GameplayModule | undefined => this._modules[type];
  getAll = () => this._modules;

  /** Get a `ToolSet` which can be passed to the `ai` SDK.
   * The `onToolCall` handler receives the new module state which it should persist.
   * State is managed via `immer`.
   */
  getToolSet(
    session: DeepReadonly<GameplaySession>,
    /** A callback which should handle persisting new state */
    onToolCall: OnToolCall,
  ): ToolSet {
    const tools: ToolSet = {};

    for (const modType of Object.keys(this._modules)) {
      const gameplayModule = this.get(modType);
      if (!gameplayModule || !gameplayModule.tools?.length) continue;

      for (const toolDef of gameplayModule.tools) {
        tools[`${modType}::${toolDef.name}`] = {
          description: toolDef.description,
          inputSchema: toolDef.parameters ? zodSchema(toolDef.parameters) : zodSchema(z.object({})),
          execute: async (input: Record<string, unknown>) => {
            const base = session.state[modType] ?? await gameplayModule.init();
            const draft = createDraft(base);

            const result = await toolDef.execute(
              input,
              {
                session,
                module: gameplayModule,
                state: draft,
              },
            );

            if (!result.success)
              return `Error: ${result.error}`;

            const newState = result.state ?? finishDraft(draft);
            onToolCall(`${modType}::${toolDef.name}`, input, modType, newState);

            return result.response ?? 'OK';
          },
        };
      }
    }

    return tools;
  }
}

export const defineGameplayModule = <
  T extends string,
  S extends z.ZodObject,
>(mod: GameplayModule<T, S>) => withAugmenters(mod);

const withAugmenters = <T extends string, S extends z.ZodObject>(mod: GameplayModule<T, S>) => ({
  ...mod,
  withTool<P extends z.ZodObject>(name: string, opts: Omit<ToolDefinition<T, P, S>, 'name'>) {
    mod.tools ??= [];
    mod.tools.push({
      name,
      ...opts,
    });
    return this;
  },
});

export const toolOk = <S>(state?: S, opts?: { response?: string }): ToolResult<S> => ({
  success: true,
  state,
  ...opts,
});

export const toolErr = (error: string): ToolResult<never> => ({ success: false, error });
