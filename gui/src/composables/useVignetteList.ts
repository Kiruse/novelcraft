import type { Ref } from 'vue';
import { select } from '~/composables/useLocalDb';

const vignettes = ref<Array<{ id: string; title: string }>>([]);
const hasMore = ref(false);

export function useVignetteList() {
  async function refresh(limit = 4) {
    const rows = await select<{ id: string; title: string }>(
      'SELECT id, title FROM local_sessions ORDER BY updated_at DESC LIMIT ?',
      [limit],
    );
    hasMore.value = rows.length > limit - 1;
    vignettes.value = rows.slice(0, limit - 1);
  }

  return {
    vignettes: vignettes as Ref<Array<{ id: string; title: string }>>,
    hasMore: hasMore as Ref<boolean>,
    refresh,
  };
}
