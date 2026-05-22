import { DeepReadonly } from "vue";

export type MaybeDeepReadonly<T> = T | DeepReadonly<T>;
export type MaybePromise<T> = T | Promise<T>;

export type UndefToNull<T extends {}> = {
  [k in keyof T]-?: undefined extends T[k] ? Exclude<T[k], undefined> | null : T[k];
}

export interface ReadableRef<T> {
  get value(): MaybeDeepReadonly<T>;
}

/** GameState rudimentarily resembles a gameplay module => module-specific data map */
export type GameState = Record<string, unknown>;

export type TypedResult<T> = { status: "ok"; data: T } | { status: "error"; error: any };

export async function unwrap<T>(result: Promise<TypedResult<T>>): Promise<T> {
  const r = await result;
  if (r.status === "ok") return r.data;
  if (typeof r.error === 'string') throw new Error(r.error);
  throw Object.assign(new Error("Backend error"), r.error);
}

export const marshal = <T extends {}>(value: T): UndefToNull<T> => value as any;
