import { select, execute } from '~/composables/useLocalDb';

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
  active: number;
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

const profiles = ref<Profile[] | undefined>(undefined);
const activeProfile = computed(() => profiles.value?.find(p => p.active) ?? null);
const ready = computed(() => profiles.value !== undefined);

let initPromise: Promise<void> | null = null;

async function refresh() {
  const rows = await select<ProfileRow>('SELECT * FROM local_profiles ORDER BY created_at');
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

  await execute(
    'INSERT INTO local_profiles (id, name, fields, active, created_at, updated_at) VALUES (?, ?, ?, 0, ?, ?)',
    [id, name, serializeFields(initialFields), now, now],
  );

  await refresh();
  return profiles.value?.find(p => p.id === id) ?? null;
}

async function update(id: string, patch: { name?: string; fields?: Record<string, string> }) {
  const updates: string[] = [];
  const values: unknown[] = [];

  updates.push("updated_at = ?");
  values.push(new Date().toISOString());

  if (patch.name !== undefined) {
    updates.push("name = ?");
    values.push(patch.name);
  }
  if (patch.fields !== undefined) {
    updates.push("fields = ?");
    values.push(serializeFields(patch.fields));
  }

  values.push(id);

  await execute(`UPDATE local_profiles SET ${updates.join(', ')} WHERE id = ?`, values);
  await refresh();
}

async function remove(id: string) {
  const wasActive = profiles.value?.find(p => p.id === id)?.active ?? false;
  await execute('DELETE FROM local_profiles WHERE id = ?', [id]);

  if (wasActive && (profiles.value?.length ?? 0) > 1) {
    const next = profiles.value?.find(p => p.id !== id);
    if (next) await setActive(next.id);
  }

  await refresh();
}

async function setActive(id: string) {
  await execute('UPDATE local_profiles SET active = 0');
  const now = new Date().toISOString();
  await execute('UPDATE local_profiles SET active = 1, updated_at = ? WHERE id = ?', [now, id]);
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
