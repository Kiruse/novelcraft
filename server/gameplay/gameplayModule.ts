import type { ConversationalData } from '@stegakir/aikit/archetypes/conversational';
import { gameSessions, moduleRuntime } from '#server/db/schema/app';
import z from 'zod';

export type GameplayModuleRuntime = typeof moduleRuntime['$inferSelect'];
export type GameplayModuleRuntimeDoc = typeof moduleRuntime['$inferInsert'];

type MaybePromise<T> = T | Promise<T>;

export interface GameplaySession {
  record: typeof gameSessions['$inferSelect'];
  modules: {
    config: unknown;
    state: unknown;
  }[];
}

export interface GameplayModule<
  T extends string = string,
  C extends z.ZodObject = z.ZodObject,
  S extends z.ZodObject = z.ZodObject,
> {
  /** Unique module identifier. */
  type: T;
  /** Config zod schema of this module. */
  config: C;
  /** State zod schema of this module. */
  state: S;
  /** Tools that the Agent can use to interface with this module. When omitted, it is assumed that this
   * module collects data from the "environment" (i.e. other gameplay modules).
   *
   * Tools should be provided to enable the agent to A) alter module state, or B) query for specific
   * information from the module.
   */
  tools?: ToolDefinition<T, z.ZodObject, C, S>[];
  /** Optional subagents that the Dungeon Master Agent may invoke in order to explore more complex tasks
   * in an isolated environment. The agent will process prompts separately, and no intermittent information
   * will leak to the user, enabling "background processing."
   */
  subagents?: Subagent[];
  /** Retrieve a formatted summary of this module's state to be injected into the module's `knowledge`. */
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

/** Register a gameplay module. Throws if a module with the same type is already registered. */
export const registerModule = <T extends string, C extends z.ZodObject, S extends z.ZodObject>(
  mod: GameplayModule<T, C, S>,
): void => {
  if (registry.has(mod.type))
    throw new Error(`Gameplay module "${mod.type}" is already registered`);
  registry.set(mod.type, mod);
};

/** Look up a registered module by type. Returns `undefined` if not found. */
export const getModule = (type: string): GameplayModule | undefined => registry.get(type);

/** Get all registered modules. */
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

// type is auto-injected into the config by the system and is taken from the module definition
export const findSessionModule = (session: GameplaySession, type: string) =>
  session.modules.find((m: any) => m.type === type);

export const toolOk = <S>(state: S): ToolResult<S> => ({ success: true, state });
export const toolErr = <S = any>(error: string): ToolResult<S> => ({ success: false, error });
