import Database from '@tauri-apps/plugin-sql';

const CREATE_TABLES = `
CREATE TABLE IF NOT EXISTS local_sessions (
  id TEXT PRIMARY KEY NOT NULL,
  story_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS local_pages (
  id TEXT PRIMARY KEY NOT NULL,
  session_id TEXT NOT NULL,
  system TEXT,
  prompt TEXT,
  response TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS local_module_runtime (
  id TEXT PRIMARY KEY NOT NULL,
  session_id TEXT NOT NULL,
  module_id TEXT NOT NULL,
  data TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS local_profiles (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  fields TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS local_onboarding (
  completed INTEGER NOT NULL DEFAULT 0
);
`;

let db: Database | null = null;

export type DbRow = Record<string, unknown>;

export async function getDb(): Promise<Database> {
  if (!db) {
    db = await Database.load('sqlite:novelcraft.db');
    await db.execute(CREATE_TABLES);
  }
  return db;
}

export async function execute(sql: string, bind?: unknown[]): Promise<number> {
  const d = await getDb();
  const result = await d.execute(sql, bind);
  return result.rowsAffected;
}

export async function select<T = Record<string, unknown>>(
  sql: string,
  bind?: unknown[],
): Promise<T[]> {
  const d = await getDb();
  return await d.select<T[]>(sql, bind);
}
