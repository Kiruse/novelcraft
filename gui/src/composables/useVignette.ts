import type { DeepReadonly, Ref } from 'vue';
import { commands } from '~/bindings';
import type { PageEntry_Serialize, SessionMeta_Serialize, Snapshot as SnapshotEntry } from '~/bindings';
import { createDefaultRegistry, type GameplaySession, type ToolCallRecord, toolCallRecordSchema } from '~/gameplay';
import type { GameState } from '~/utils';
import { marshal, unwrap } from '~/utils';

type LoadingState = 'loading' | 'ready' | 'error';

export type Vignette = ReturnType<typeof useVignette>;

export type VignetteMeta = SessionMeta_Serialize;

export type VignettePage = PageEntry_Serialize;

export interface ForkOpts {
  pageIndex: number;
  system?: string | null;
  prompt?: string | null;
}

export interface UpdateOpts {
  pageIndex: number;
  system?: string | null;
  prompt?: string | null;
  response?: string | null;
}

export type PromptUpdater = (response: string, toolCalls: ToolCallRecord[], state: GameState) => Promise<void>;

interface Snapshot {
  id: string;
  sessionId: string;
  pageIndex: number;
  data: GameState;
}

const SNAPSHOT_CHECKPOINT_INTERVAL = 100;

function snapshotToEntry(s: Snapshot, sessionId: string): SnapshotEntry {
  return { version: 1, id: s.id, sessionId, pageIndex: s.pageIndex, data: s.data };
}

function entryToSnapshot(e: SnapshotEntry): Snapshot {
  return { id: e.id, sessionId: e.sessionId, pageIndex: e.pageIndex, data: e.data as GameState };
}

export function useVignette(sessionId: DeepReadonly<Ref<string, any>>) {
  const now = new Date();
  const status = ref<LoadingState>('loading');
  const meta = ref<VignetteMeta>({
    id: sessionId.value,
    version: 1,
    title: '',
    createdAt: now.toISOString(),
    updatedAt: now.toISOString(),
  });
  const pages = ref<VignettePage[]>([]);
  const error = ref<string | undefined>();
  const snapshot = ref<Snapshot>({
    id: '',
    sessionId: sessionId.value,
    pageIndex: 0,
    data: {},
  });

  const registry = createDefaultRegistry();

  const getSession = (): GameplaySession => ({
    sessionId: sessionId.value,
    storyId: meta.value.storyId ?? undefined,
    state: snapshot.value.data,
  });

  async function replay(
    _snapshot: Snapshot,
    _pages: VignettePage[],
  ) {
    const session = getSession();

    for (const page of _pages.slice(_snapshot.pageIndex + 1)) {
      if (!page.toolCalls) continue;

      const toolCalls = toolCallRecordSchema.array().parse(JSON.parse(page.toolCalls));
      for (const toolCall of toolCalls) {
        const [modType, ...toolTypes] = toolCall.tool.split('::');
        const toolName = toolTypes.join('::');

        const result = await registry.executeTool(
          session, modType, toolName, toolCall.params,
          (mt) => _snapshot.data[mt],
        );
        _snapshot.data[modType] = result.newState;
      }
    }
  }

  async function load(id: string) {
    return await unwrap(commands.sessionLoad(id));
  }

  async function save() {
    const now = new Date().toISOString();
    meta.value.updatedAt = now;
    await unwrap(commands.sessionSaveMeta(marshal(meta.value)));
  }

  /** Push a new page to the vignette.
   * @returns an updater that can be called once the AI has finished generating its response.
   */
  async function push({ prompt, system }: { prompt?: string, system?: string }): Promise<PromptUpdater> {
    const sid = sessionId.value;
    const pageIndex = pages.value.length;

    const page: VignettePage = {
      id: crypto.randomUUID(),
      sessionId: sid,
      system,
      prompt,
    };

    await unwrap(commands.sessionUpsertPage({ session_id: sid, page_index: null, page: marshal(page) }));
    pages.value.push(page);

    return async (response, toolCalls, data) => {
      const _snapshot: Snapshot = {
        id: crypto.randomUUID(),
        sessionId: sid,
        pageIndex,
        data,
      };

      page.response = response;

      if (toolCalls.length) {
        const toolCallsSerialized = page.toolCalls = JSON.stringify(toolCalls);
        await unwrap(commands.sessionUpsertPage({ session_id: sid, page_index: pageIndex, page: marshal(page) }));
        await unwrap(commands.sessionSaveHeadSnapshot(sid, snapshotToEntry(_snapshot, sid)));
        snapshot.value = _snapshot;
      } else {
        await unwrap(commands.sessionUpsertPage({ session_id: sid, page_index: pageIndex, page: marshal(page) }));

        if (pageIndex > 0 && pageIndex % SNAPSHOT_CHECKPOINT_INTERVAL === 0) {
          await unwrap(commands.sessionSaveCheckpoint(sid, snapshotToEntry(_snapshot, sid)));
        }

        await unwrap(commands.sessionSaveHeadSnapshot(sid, snapshotToEntry({ ..._snapshot, id: snapshot.value.id }, sid)));
        snapshot.value = { ..._snapshot, id: snapshot.value.id };
      }
    };
  }

  /** Fork from the given page index. Expects to recompute the response.
   * @returns an updater that can be called once the AI has finished generating its response.
   */
  async function fork({ pageIndex, system, prompt }: ForkOpts): Promise<PromptUpdater> {
    let _pages = pages.value.slice();
    const page = _pages[pageIndex];
    if (!page) throw new RangeError(`Page index ${pageIndex} out of bounds`);

    pages.value = _pages = _pages.slice(0, pageIndex);

    await unwrap(commands.sessionTruncatePages(sessionId.value, pageIndex));
    await unwrap(commands.sessionDeleteCheckpointsFrom(sessionId.value, pageIndex));
    await unwrap(commands.sessionDeleteHeadSnapshot(sessionId.value));

    let _snapshot: Snapshot;
    if (pageIndex > 0) {
      const found = await unwrap(commands.sessionFindSnapshotBefore(sessionId.value, pageIndex - 1));
      if (found) {
        _snapshot = entryToSnapshot(found);
      } else {
        _snapshot = { id: crypto.randomUUID(), sessionId: sessionId.value, pageIndex: 0, data: {} };
      }
    } else {
      _snapshot = { id: crypto.randomUUID(), sessionId: sessionId.value, pageIndex: 0, data: {} };
    }

    await replay(_snapshot, _pages);
    snapshot.value = _snapshot;

    return await push({
      system: system === null ? undefined : system ?? page.system ?? undefined,
      prompt: prompt === null ? undefined : prompt ?? page.prompt ?? undefined,
    });
  }

  async function update({ pageIndex, system, prompt, response }: UpdateOpts): Promise<void> {
    const page = pages.value[pageIndex];
    if (!page) throw new RangeError(`Page index ${pageIndex} out of bounds`);

    const newSystem = system === null ? null : system ?? page.system ?? null;
    const newPrompt = prompt === null ? null : prompt ?? page.prompt ?? null;
    const newResponse = response === null ? null : response ?? page.response ?? null;

    await unwrap(commands.sessionUpsertPage({
      session_id: sessionId.value,
      page_index: pageIndex,
      page: marshal({ ...page, system: newSystem, prompt: newPrompt, response: newResponse }),
    }));

    page.system = newSystem ?? undefined;
    page.prompt = newPrompt ?? undefined;
    page.response = newResponse ?? undefined;
  }

  watch(sessionId, async (newId, _oldId, onCleanup) => {
    let mounted = true;

    onCleanup(() => {
      mounted = false;
    });

    try {
      status.value = 'loading';
      const { pages: vignettePages, meta: vignetteMeta } = await load(newId);
      if (!mounted) return;

      const headResult = await unwrap(commands.sessionGetHeadSnapshot(newId));
      if (!mounted) return;

      let _snapshot: Snapshot;
      if (headResult) {
        _snapshot = entryToSnapshot(headResult);
      } else {
        _snapshot = { id: '', sessionId: newId, pageIndex: 0, data: {} };
      }

      await replay(_snapshot, vignettePages);
      if (!mounted) return;

      meta.value = vignetteMeta;
      pages.value = vignettePages;
      snapshot.value = _snapshot;
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
    snapshot: readonly(snapshot),
    error: readonly(error),
    save,
    push,
    fork,
    update,
    getGameplaySession: (): GameplaySession => ({
      sessionId: sessionId.value,
      storyId: meta.value.storyId ?? undefined,
      state: snapshot.value.data,
    }),
  };
}
