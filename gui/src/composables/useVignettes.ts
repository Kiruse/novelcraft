import { execute, select } from '~/composables/useLocalDb';

export interface VignetteRow {
  id: string;
  title: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

const recent = ref<VignetteRow[]>([]);
const hasMore = ref(false);

export function useVignettes() {
  const vignettes = ref<VignetteRow[] | undefined>(undefined);

  async function refresh() {
    await refreshRecent();
    if (vignettes) await refreshAll();
  }

  async function refreshRecent() {
    const rows = await select<VignetteRow>(
      'SELECT id, title, description, created_at, updated_at FROM local_sessions ORDER BY updated_at DESC LIMIT 4',
    );
    hasMore.value = rows.length > 3;
    recent.value = rows.slice(0, 3);
  }

  async function refreshAll() {
    vignettes.value = await select<VignetteRow>(
      'SELECT id, title, description, created_at, updated_at FROM local_sessions ORDER BY updated_at DESC',
    );
  }

  async function create() {
    const id = crypto.randomUUID();
    const now = new Date().toISOString();
    await execute(
      'INSERT INTO local_sessions (id, story_id, title, created_at, updated_at) VALUES (?, ?, ?, ?, ?)',
      [id, `vignette:${id}`, 'Untitled Vignette', now, now],
    );
    refresh();
    return id;
  }

  async function remove(id: string) {
    await execute(
      `BEGIN TRANSACTION;
      DELETE FROM local_pages WHERE id = ?;
      DELETE FROM local_sessions WHERE id = ?;
      COMMIT;`,
      [id, id],
    );
    refresh();
  }

  return {
    vignettes: readonly(vignettes),
    recent: readonly(recent),
    hasMore: readonly(hasMore),
    create,
    remove,
    refresh,
    loadVignettes: refreshAll,
  };
}
