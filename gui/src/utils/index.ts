import { DeepReadonly } from "vue";

export type MaybeDeepReadonly<T> = T | DeepReadonly<T>;
export type MaybePromise<T> = T | Promise<T>;

export interface ReadableRef<T> {
  get value(): MaybeDeepReadonly<T>;
}

/** GameState rudimentarily resembles a gameplay module => module-specific data map */
export type GameState = Record<string, unknown>;
