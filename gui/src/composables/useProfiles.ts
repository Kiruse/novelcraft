import type { Profile } from '~/bindings';
export type { Profile };
import { commands } from '~/bindings';
import { unwrap } from '~/utils';

const DEFAULT_FIELDS: Record<string, string> = {
  name: '',
  appearance: '',
  personality: '',
  interests: '',
  'favorite color': '',
};

const profiles = ref<Profile[] | undefined>(undefined);
const activeId = ref<string | null>(null);
const activeProfile = computed(() => profiles.value?.find(p => p.id === activeId.value) ?? null);
const ready = computed(() => profiles.value !== undefined);

let initPromise: Promise<void> | null = null;

async function refresh() {
  const result = await unwrap(commands.profileList());
  profiles.value = result.profiles;
  activeId.value = result.active_id ?? result.profiles[0]?.id ?? null;
}

async function create(name: string, fields?: Record<string, string>): Promise<Profile | null> {
  const now = new Date().toISOString();
  const id = crypto.randomUUID();
  const initialFields = fields ?? { ...DEFAULT_FIELDS };

  await unwrap(commands.profileCreate(id, name, initialFields, now));

  await refresh();
  return profiles.value?.find(p => p.id === id) ?? null;
}

async function update(id: string, patch: { name?: string; fields?: Record<string, string> }) {
  const now = new Date().toISOString();
  const profile = profiles.value?.find(p => p.id === id);
  if (!profile) return;

  await unwrap(commands.profileUpdate(id, patch.name ?? profile.name, patch.fields ?? profile.fields, now));
  await refresh();
}

async function remove(id: string) {
  const wasActive = activeId.value === id;
  await unwrap(commands.profileDelete(id));

  if (wasActive && (profiles.value?.length ?? 0) > 1) {
    const next = profiles.value?.find(p => p.id !== id);
    if (next) await setActive(next.id);
  }

  await refresh();
}

async function setActive(id: string) {
  await unwrap(commands.profileSetActive(id));
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
        } else if (!activeId.value) {
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
    defaultFields: { ...DEFAULT_FIELDS },
  };
}
