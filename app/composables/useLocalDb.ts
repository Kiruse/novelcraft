import { PowerSyncDatabase } from '@powersync/web';
import { wrapPowerSyncWithDrizzle } from '@powersync/drizzle-driver';
import { DrizzleAppSchema } from '@powersync/drizzle-driver';
import { drizzleSchema } from '#shared/db/localSchema';

const AppSchema = new DrizzleAppSchema(drizzleSchema);

let powerSyncDb: PowerSyncDatabase | null = null;
let drizzle: ReturnType<typeof wrapPowerSyncWithDrizzle> | null = null;

function getPowerSyncDb(): PowerSyncDatabase {
  if (!powerSyncDb) {
    powerSyncDb = new PowerSyncDatabase({
      schema: AppSchema,
      database: {
        dbFilename: 'novelcraft.db',
      },
    });
  }
  return powerSyncDb;
}

export function useLocalDb() {
  if (!drizzle) {
    drizzle = wrapPowerSyncWithDrizzle(getPowerSyncDb(), {
      schema: drizzleSchema,
    });
  }
  return drizzle;
}

export async function initLocalDb() {
  const db = getPowerSyncDb();
  await db.init();
}
