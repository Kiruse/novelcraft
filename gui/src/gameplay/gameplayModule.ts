import type { ConversationalData } from '@stegakir/aikit/archetypes/conversational';
import z from 'zod';

export interface GameplayModuleRuntime {
  id: number;
  gameSessionId: number;
  moduleId: string;
  data: Record<string, unknown>;
}

export interface GameplayModuleRuntimeDoc {
  gameSessionId: number;
  moduleId: string;
  data: Record<string, unknown>;
}

type MaybePromise<T> = T | Promise<T>;

export interface GameplaySession {
  modules: {
    type: string;
    config: unknown;
    state: unknown;
  }[];
}

export interface GameplayModule<
  T extends string = string,
  C extends z.ZodObject = z.ZodObject,
  S extends z.ZodObject = z.ZodObject,
> {
  type: T;
  config: C;
  state: S;
  tools?: ToolDefinition<T, z.ZodObject, C, S>[];
  subagents?: Subagent[];
  getKnowledge(ctx: GameplayModuleContext<T, C, S>): object;
}

export interface ToolDefinition<T extends string, P extends z.ZodObject, C extends z.ZodObject, S extends z.ZodObject> {
  name: string;
  description: string;
  parameters?: P;
  execute(params: z.infer<P>, context: GameplayModuleContext<T, C, S>): MaybePromise<ToolResult<z.infer<S>>>;
}

export interface GameplayModuleContext<T extends string, C extends z.ZodObject, S extends z.ZodObject> {
  session: GameplaySession;
  module: GameplayModule<T, C, S>;
  config: z.infer<C>;
  state: z.infer<S>;
}

export type ToolResult<S> = { success: true; state: S } | { success: false; error: string };

export interface Subagent {
  name: string;
  summary: string;
  prompt: ConversationalData;
}

const registry = new Map<string, GameplayModule>();

export const registerModule = <T extends string, C extends z.ZodObject, S extends z.ZodObject>(
  mod: GameplayModule<T, C, S>,
): void => {
  if (registry.has(mod.type))
    throw new Error(`Gameplay module "${mod.type}" is already registered`);
  registry.set(mod.type, mod);
};

export const getModule = (type: string): GameplayModule | undefined => registry.get(type);

export const getAllModules = (): ReadonlyMap<string, GameplayModule> => registry;

export const defineGameplayModule = <
  T extends string,
  C extends z.ZodObject,
  S extends z.ZodObject,
>(mod: GameplayModule<T, C, S>) => withAugmenters(mod);

const withAugmenters = <T extends string, C extends z.ZodObject, S extends z.ZodObject>(mod: GameplayModule<T, C, S>) => ({
  ...mod,
  withTool<P extends z.ZodObject>(name: string, opts: Omit<ToolDefinition<T, P, C, S>, 'name'>) {
    mod.tools ??= [];
    mod.tools.push({
      name,
      ...opts,
    });
    return this;
  },
});

export const findSessionModule = (session: GameplaySession, type: string) =>
  session.modules.find((m) => m.type === type);

export const toolOk = <S>(state: S): ToolResult<S> => ({ success: true, state });
export const toolErr = (error: string): ToolResult<never> => ({ success: false, error });
