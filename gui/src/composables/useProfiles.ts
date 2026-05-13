import { eq } from 'drizzle-orm';
import { db, localProfiles } from '~/db';

const DEFAULT_FIELDS: Record<string, string> = {
  name: '',
  appearance: '',
  personality: '',
  interests: '',
  'favorite color': '',
};

const MAX_PROFILES = 5;

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

const profiles = ref<Profile[] | undefined>(undefined);
const activeProfile = computed(() => profiles.value?.find(p => p.active) ?? null);
const ready = computed(() => profiles.value !== undefined);

let initPromise: Promise<void> | null = null;

async function refresh() {
  const rows = await db.select().from(localProfiles).orderBy(localProfiles.createdAt);
  profiles.value = rows.map(r => ({
    id: r.id,
    name: r.name,
    fields: parseFields(r.fields),
    active: r.active === 1,
    createdAt: r.createdAt,
    updatedAt: r.updatedAt,
  }));
}

async function create(name: string, fields?: Record<string, string>): Promise<Profile | null> {
  if ((profiles.value?.length ?? 0) >= MAX_PROFILES) return null;

  const now = new Date().toISOString();
  const id = crypto.randomUUID();
  const initialFields = fields ?? { ...DEFAULT_FIELDS };

  await db.insert(localProfiles).values({
    id,
    name,
    fields: serializeFields(initialFields),
    active: 0,
    createdAt: now,
    updatedAt: now,
  });

  await refresh();
  return profiles.value?.find(p => p.id === id) ?? null;
}

async function update(id: string, patch: { name?: string; fields?: Record<string, string> }) {
  const set: Partial<typeof localProfiles.$inferInsert> & { updatedAt: string } = {
    updatedAt: new Date().toISOString(),
  };

  if (patch.name !== undefined) set.name = patch.name;
  if (patch.fields !== undefined) set.fields = serializeFields(patch.fields);

  await db.update(localProfiles).set(set).where(eq(localProfiles.id, id));
  await refresh();
}

async function remove(id: string) {
  const wasActive = profiles.value?.find(p => p.id === id)?.active ?? false;
  await db.delete(localProfiles).where(eq(localProfiles.id, id));

  if (wasActive && (profiles.value?.length ?? 0) > 1) {
    const next = profiles.value?.find(p => p.id !== id);
    if (next) await setActive(next.id);
  }

  await refresh();
}

async function setActive(id: string) {
  await db.update(localProfiles).set({ active: 0 });
  const now = new Date().toISOString();
  await db.update(localProfiles).set({ active: 1, updatedAt: now }).where(eq(localProfiles.id, id));
  await refresh();
}

async function init() {
  if (!initPromise) {
    initPromise = (async () => {
      try {
        await refresh();
        if (profiles.value!.length === 0) {
          await create('Default', { ...DEFAULT_FIELDS });
          await setActive(profiles.value![0]!.id);
        } else if (!profiles.value!.some(p => p.active)) {
          await setActive(profiles.value![0]!.id);
        }
      } catch {
        initPromise = null;
      }
    })();
  }
  await initPromise;
}

export function useProfiles() {
  onMounted(() => {
    if (!initPromise) init();
  });

  return {
    profiles: readonly(profiles),
    activeProfile,
    ready,
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
