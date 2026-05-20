import { eq, desc } from 'drizzle-orm';
import { db, localSessions, localStateSnapshots, localPages } from '~/db';

export interface VignetteRow {
  id: string;
  title: string;
  description: string | null;
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
    const rows = await db.select({
      id: localSessions.id,
      title: localSessions.title,
      description: localSessions.description,
      createdAt: localSessions.createdAt,
      updatedAt: localSessions.updatedAt,
    }).from(localSessions).orderBy(desc(localSessions.updatedAt)).limit(4);

    hasMore.value = rows.length > 3;
    recent.value = rows.slice(0, 3);
  }

  async function refreshAll() {
    vignettes.value = await db.select({
      id: localSessions.id,
      title: localSessions.title,
      description: localSessions.description,
      createdAt: localSessions.createdAt,
      updatedAt: localSessions.updatedAt,
    }).from(localSessions).orderBy(desc(localSessions.updatedAt));
  }

  async function create() {
    const id = crypto.randomUUID();
    const now = new Date().toISOString();

    await db.transaction(async (tx) => {
      await tx.insert(localSessions).values({
        id,
        storyId: `vignette:${id}`,
        title: 'Untitled Vignette',
        createdAt: now,
        updatedAt: now,
      });
      await tx.insert(localStateSnapshots).values({
        id: crypto.randomUUID(),
        sessionId: id,
        pageIndex: 0,
        data: '{}',
        createdAt: now,
      });
    });

    await refresh();
    return id;
  }

  async function remove(id: string) {
    await db.transaction(async (tx) => {
      await tx.delete(localPages).where(eq(localPages.sessionId, id));
      await tx.delete(localStateSnapshots).where(eq(localStateSnapshots.sessionId, id));
      await tx.delete(localSessions).where(eq(localSessions.id, id));
    });
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
