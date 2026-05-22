import { commands } from '~/bindings';
import { unwrap } from '~/utils';

export interface VignetteRow {
  id: string;
  title: string;
  description?: string | null;
  createdAt: string;
  updatedAt: string;
}

const recent = ref<VignetteRow[]>([]);
const hasMore = ref(false);
const vignettes = ref<VignetteRow[] | undefined>(undefined);

export function useVignettes() {
  async function refresh() {
    await refreshRecent();
    if (vignettes.value !== undefined) await refreshAll();
  }

  async function refreshRecent() {
    const all = await unwrap(commands.sessionList());
    hasMore.value = all.length > 3;
    recent.value = all.slice(0, 3);
  }

  async function refreshAll() {
    vignettes.value = await unwrap(commands.sessionList());
  }

  async function create() {
    const result = await unwrap(commands.sessionCreate({ title: 'Untitled Vignette', story_id: null }));
    await refresh();
    return result;
  }

  async function remove(id: string) {
    await unwrap(commands.sessionDelete(id));
    await refresh();
  }

  return {
    vignettes: readonly(vignettes),
    recent: readonly(recent),
    hasMore: readonly(hasMore),
    create,
    remove,
    refresh,
    loadVignettes: async () => {
      await refreshAll();
    },
  };
}
