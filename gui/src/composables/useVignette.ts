import { DeepReadonly, Ref } from "vue";
import { execute, select } from "./useLocalDb";
import { getErrorDisplay } from "~/utils/msgUtils";
import { useToast } from "./useToast";

type LoadingState = 'loading' | 'ready' | 'error';

interface SessionRow {
  id: string;
  title: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export type Vignette = ReturnType<typeof useVignette>;

export interface VignetteMeta {
  title: string;
  disposition: string;
  createdAt: Date;
  updatedAt: Date;
}

export interface VignettePage {
  id: string;
  session_id: string;
  system?: string | null;
  prompt?: string | null;
  response?: string | null;
}

export interface ForkOpts {
  pageIndex: number;
  system?: string | null;
  prompt?: string | null;
  response?: string | null;
}

export function useVignette(id: DeepReadonly<Ref<string, any>>) {
  const now = new Date();
  const status = ref<LoadingState>('loading');
  const meta = ref<VignetteMeta>({ title: '', disposition: '', createdAt: now, updatedAt: now });
  const pages = ref<VignettePage[]>([]);
  const error = ref<string | undefined>();

  const toast = useToast();

  async function load(id: string) {
    const sessionRows = await select<SessionRow>('SELECT * FROM local_sessions WHERE id = ?', [id]);
    const session = sessionRows[0];
    if (!session) return null;

    const pages = await select<VignettePage>('SELECT * FROM local_pages WHERE session_id = ?', [session.id]);

    return {
      title: session.title,
      disposition: session.description ?? '',
      createdAt: new Date(session.created_at),
      updatedAt: new Date(session.updated_at),
      pages,
    };
  }

  async function save() {
    const now = new Date();
    meta.value.updatedAt = now;
    await execute(
      `UPDATE local_sessions SET title = ?, description = ?, updated_at = ? WHERE id = ?`,
      [meta.value.title, meta.value.disposition, now.toISOString(), id.value],
    );
  }

  /** Push a new page to the end of the vignette. At this point does not have a response yet. */
  async function push(prompt?: string) {
    const ts = new Date().toISOString();
    const pageId = crypto.randomUUID();
    pages.value.push({
      id: pageId,
      session_id: id.value,
      prompt,
    });

    await execute(
      `BEGIN TRANSACTION;
      UPDATE local_sessions SET updated_at = ?;
      INSERT INTO local_pages(id, session_id, prompt, created_at) VALUES(?, ?, ?, ?);
      COMMIT;`,
      [ts, pageId, id.value, prompt, ts],
    );
  }

  async function fork({ pageIndex, system, prompt, response }: ForkOpts) {
    const ts = new Date().toISOString();
    const page = pages.value[pageIndex];
    if (!page) return;

    // All pages after the updated page are lost, for now
    try {
      const truncateIds = pages.value.slice(pageIndex + 1).map(p => p.id);
      if (truncateIds.length > 0) {
        await execute(
          `BEGIN TRANSACTION;
          DELETE FROM local_pages WHERE id IN (${truncateIds.map(() => '?').join(', ')});
          UPDATE local_sessions SET updated_at = ?;
          UPDATE local_pages SET system = ?, prompt = ?, response = ? WHERE id = ?;
          COMMIT;`,
          [...truncateIds, ts, system, prompt, response, page.id],
        );
      } else {
        await execute(
          `BEGIN TRANSACTION;
          UPDATE local_sessions SET updated_at = ?;
          UPDATE local_pages SET system = ?, prompt = ?, response = ? WHERE id = ?;
          COMMIT;`,
          [ts, system, prompt, response, page.id],
        );
      }
    } catch (err) {
      toast.error(getErrorDisplay(err));
      console.error('useVignette().fork():', err);
      return;
    }

    pages.value = [
      ...pages.value.slice(0, pageIndex),
      {
        ...page,
        system,
        prompt,
        response,
      },
    ];
  }

  watch(id, async (newId, _oldId, onCleanup) => {
    let mounted = true;

    onCleanup(() => {
      mounted = false;
    });

    try {
      status.value = 'loading';
      const data = await load(newId);
      if (!mounted) return;

      const now = new Date();

      meta.value = {
        title: data?.title || 'Untitled Vignette',
        disposition: data?.disposition ?? '',
        createdAt: now,
        updatedAt: now,
      };
      pages.value = data?.pages ?? [];
      status.value = 'ready';
    } catch (err) {
      if (!mounted) return;
      status.value = 'error';
      error.value = err instanceof Error ? err.message : err + '';
    }
  }, { immediate: true });

  return {
    status: readonly(status),
    meta,
    pages: readonly(pages),
    error: readonly(error),
    save,
    push,
    fork,
  };
}
