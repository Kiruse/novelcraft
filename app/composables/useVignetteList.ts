import type { Ref } from 'vue';

const vignettes = ref<Array<{ id: string; title: string }>>([]);
const hasMore = ref(false);

export function useVignetteList() {
  async function refresh(limit = 4) {
    const db = useLocalDb();
    const { localSessions } = await import('#shared/db/localSchema');
    const { desc } = await import('drizzle-orm');
    const rows = await db
      .select({ id: localSessions.id, title: localSessions.title })
      .from(localSessions)
      .orderBy(desc(localSessions.updatedAt))
      .limit(limit)
      .all();
    hasMore.value = rows.length > limit - 1;
    vignettes.value = rows.slice(0, limit - 1);
  }

  return {
    vignettes: vignettes as Ref<Array<{ id: string; title: string }>>,
    hasMore: hasMore as Ref<boolean>,
    refresh,
  };
}
