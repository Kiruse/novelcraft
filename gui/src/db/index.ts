import { drizzle } from 'drizzle-orm/sqlite-proxy';
import Database from '@tauri-apps/plugin-sql';
import * as schema from './schema';

const migrationEntries = Object.entries(
  import.meta.glob('../../drizzle/*.sql?raw', {
    eager: true,
    import: 'default',
  }) as Record<string, string>,
)
  .filter(([path]) => !path.includes('/meta/'))
  .sort(([a], [b]) => a.localeCompare(b))
  .map(([path, sql]) => ({
    name: path.split('/').pop()!,
    sql,
  }));

function splitStatements(migrationSql: string): string[] {
  return migrationSql
    .split('--> statement-breakpoint')
    .map(s => s.trim())
    .filter(s => s.length > 0);
}

async function runMigrations(conn: Database) {
  await conn.execute(
    `CREATE TABLE IF NOT EXISTS _migrations (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL UNIQUE,
      applied_at TEXT NOT NULL
    )`,
  );

  const appliedRows = await conn.select<{ name: string }[]>(
    'SELECT name FROM _migrations ORDER BY id',
  );
  const appliedSet = new Set(appliedRows.map(r => r.name));

  if (appliedSet.size === 0 && migrationEntries.length > 0) {
    const existing = await conn.select<{ name: string }[]>(
      "SELECT name FROM sqlite_master WHERE type='table' AND name='local_stories'",
    );

    if (existing.length > 0) {
      for (const m of migrationEntries) {
        await conn.execute(
          'INSERT INTO _migrations (name, applied_at) VALUES (?, ?)',
          [m.name, new Date().toISOString()],
        );
      }
      return;
    }
  }

  for (const migration of migrationEntries) {
    if (appliedSet.has(migration.name)) continue;

    const statements = splitStatements(migration.sql);
    for (const stmt of statements) {
      await conn.execute(stmt);
    }

    await conn.execute(
      'INSERT INTO _migrations (name, applied_at) VALUES (?, ?)',
      [migration.name, new Date().toISOString()],
    );
  }
}

let sqlite: Database | null = null;

async function getSqlite(): Promise<Database> {
  if (!sqlite) {
    sqlite = await Database.load('sqlite:novelcraft.db');
    await runMigrations(sqlite);
  }
  return sqlite;
}

export const db = drizzle(
  async (sql, params, method) => {
    const conn = await getSqlite();

    if (method === 'run') {
      await conn.execute(sql, params);
      return { rows: [] };
    }

    const rows: Record<string, unknown>[] = await conn.select(sql, params);
    const mapped = rows.map((row) => Object.values(row));

    if (method === 'get') {
      return { rows: mapped[0] ?? [] };
    }

    return { rows: mapped };
  },
  { schema, logger: true },
);

export type SQLiteTxCallback = Parameters<typeof db['transaction']>[0]
export type SQLiteTx = Parameters<SQLiteTxCallback>[0];

export { schema };
export * from './schema';
