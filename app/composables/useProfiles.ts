import { localProfiles } from '#shared/db/localSchema';
import { eq } from 'drizzle-orm';

const DEFAULT_FIELDS: Record<string, string> = {
  name: '',
  appearance: '',
  personality: '',
  interests: '',
  'favorite color': '',
};

const MAX_PROFILES = 5;

export interface ProfileRow {
  id: string;
  name: string;
  fields: string;
  active: boolean;
  createdAt: string;
  updatedAt: string;
}

export type Profile = {
  id: string;
  name: string;
  fields: Record<string, string>;
  active: boolean;
  createdAt: string;
  updatedAt: string;
};

function parseFields(raw: string): Record<string, string> {
  try {
    return JSON.parse(raw) as Record<string, string>;
  } catch {
    return { ...DEFAULT_FIELDS };
  }
}

function serializeFields(fields: Record<string, string>): string {
  return JSON.stringify(fields);
}

export function useProfiles() {
  const profiles = ref<Profile[]>([]);
  const activeProfile = computed(() => profiles.value.find(p => p.active) ?? null);

  async function refresh() {
    const db = useLocalDb();
    const rows = await db.select().from(localProfiles).all();
    profiles.value = rows.map(r => ({
      ...r,
      fields: parseFields(r.fields),
    }));
  }

  async function create(name: string, fields?: Record<string, string>): Promise<Profile | null> {
    if (profiles.value.length >= MAX_PROFILES) return null;

    const db = useLocalDb();
    const now = new Date().toISOString();
    const id = crypto.randomUUID();
    const initialFields = fields ?? { ...DEFAULT_FIELDS };

    await db.insert(localProfiles).values({
      id,
      name,
      fields: serializeFields(initialFields),
      active: false,
      createdAt: now,
      updatedAt: now,
    }).run();

    await refresh();
    return profiles.value.find(p => p.id === id) ?? null;
  }

  async function update(id: string, patch: { name?: string; fields?: Record<string, string> }) {
    const db = useLocalDb();
    const updates: Record<string, unknown> = { updatedAt: new Date().toISOString() };
    if (patch.name !== undefined) updates.name = patch.name;
    if (patch.fields !== undefined) updates.fields = serializeFields(patch.fields);

    await db.update(localProfiles).set(updates).where(eq(localProfiles.id, id)).run();
    await refresh();
  }

  async function remove(id: string) {
    const db = useLocalDb();
    const wasActive = profiles.value.find(p => p.id === id)?.active ?? false;
    await db.delete(localProfiles).where(eq(localProfiles.id, id)).run();

    if (wasActive && profiles.value.length > 1) {
      const next = profiles.value.find(p => p.id !== id);
      if (next) await setActive(next.id);
    }

    await refresh();
  }

  async function setActive(id: string) {
    const db = useLocalDb();
    await db.update(localProfiles).set({ active: false }).run();
    await db.update(localProfiles).set({ active: true, updatedAt: new Date().toISOString() }).where(eq(localProfiles.id, id)).run();
    await refresh();
  }

  async function init() {
    try {
      await refresh();
      if (profiles.value.length === 0) {
        await create('Default', { ...DEFAULT_FIELDS });
        await setActive(profiles.value[0]!.id);
      } else if (!profiles.value.some(p => p.active)) {
        await setActive(profiles.value[0]!.id);
      }
    } catch {
      // local DB not available
    }
  }

  return {
    profiles: readonly(profiles),
    activeProfile,
    refresh,
    create,
    update,
    remove,
    setActive,
    init,
    maxProfiles: MAX_PROFILES,
    defaultFields: { ...DEFAULT_FIELDS },
  };
}
