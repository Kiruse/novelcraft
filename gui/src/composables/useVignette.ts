import { eq, desc, and, lte, inArray, sql, gte } from 'drizzle-orm';
import type { DeepReadonly, Ref } from "vue";
import { db, localSessions, localPages, localStateSnapshots, SQLiteTx } from "~/db";
import { createDefaultRegistry, type GameplaySession, type ToolCallRecord, toolCallRecordSchema } from "~/gameplay";
import type { GameState } from "~/utils";

type LoadingState = 'loading' | 'ready' | 'error';

export type Vignette = ReturnType<typeof useVignette>;

export interface VignetteMeta {
  title: string;
  storyId: string;
  disposition: string;
  createdAt: Date;
  updatedAt: Date;
}

export interface VignettePage {
  id: string;
  sessionId: string;
  system?: string;
  prompt?: string;
  response?: string;
  toolCalls?: string;
}

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
  createdAt: Date;
}

/** Interval between snapshot checkpoints, i.e. these snapshots are kept to speed up
 * snapshot replay
 */
const SNAPSHOT_CHECKPOINT_INTERVAL = 100;

export function useVignette(sessionId: DeepReadonly<Ref<string, any>>) {
  const now = new Date();
  const status = ref<LoadingState>('loading');
  const meta = ref<VignetteMeta>({ title: '', storyId: '', disposition: '', createdAt: now, updatedAt: now });
  const pages = ref<VignettePage[]>([]);
  const error = ref<string | undefined>();
  const snapshot = ref<Snapshot>({
    id: '',
    sessionId: sessionId.value,
    pageIndex: 0,
    data: {},
    createdAt: new Date(),
  });

  const registry = createDefaultRegistry();

  const getSession = (): GameplaySession => ({
    sessionId: sessionId.value,
    storyId: meta.value.storyId,
    state: snapshot.value.data,
  });

  async function replay(
    snapshot: Snapshot,
    pages: VignettePage[],
  ) {
    const session = getSession();

    for (const page of pages.slice(snapshot.pageIndex + 1)) {
      if (!page.toolCalls) continue;

      const toolCalls = toolCallRecordSchema.array().parse(JSON.parse(page.toolCalls));
      for (const toolCall of toolCalls) {
        const [modType, ...toolTypes] = toolCall.tool.split('::');
        const toolName = toolTypes.join('::');

        const result = await registry.executeTool(
          session, modType, toolName, toolCall.params,
          (mt) => snapshot.data[mt],
        );
        snapshot.data[modType] = result.newState;
      }
    }
  }

  /** Load the youngest snapshot before the given page index, including a snapshot
   * on the page index itself.
   */
  async function loadSnapshot(pageIndex: number): Promise<Snapshot> {
    const [snapshot] = await db.select({
      id: localStateSnapshots.id,
      sessionId: localStateSnapshots.sessionId,
      pageIndex: localStateSnapshots.pageIndex,
      data: localStateSnapshots.data,
      createdAt: localStateSnapshots.createdAt,
    }).from(localStateSnapshots)
      .where(and(
        eq(localStateSnapshots.sessionId, sessionId.value),
        lte(localStateSnapshots.pageIndex, pageIndex),
      ))
      .orderBy(desc(localStateSnapshots.pageIndex))
      .limit(1);
    if (!snapshot) throw new Error(`No snapshot found before page index ${pageIndex}`);
    return {
      id: snapshot.id,
      sessionId: snapshot.sessionId,
      pageIndex: snapshot.pageIndex,
      data: JSON.parse(snapshot.data),
      createdAt: new Date(snapshot.createdAt),
    };
  }

  async function load(id: string) {
    const sessionRows = await db.select().from(localSessions).where(eq(localSessions.id, id));
    const session = sessionRows[0];
    if (!session) throw new Error(`Session ${id} not found`);

    const loadedPages = await db.select()
      .from(localPages)
      .where(eq(localPages.sessionId, session.id));

    return {
      title: session.title,
      storyId: session.storyId,
      disposition: session.description ?? '',
      createdAt: new Date(session.createdAt),
      updatedAt: new Date(session.updatedAt),
      pages: loadedPages.map((p): VignettePage => ({
        id: p.id,
        sessionId: p.sessionId,
        system: p.system ?? undefined,
        prompt: p.prompt ?? undefined,
        response: p.response ?? undefined,
        toolCalls: p.toolCalls ?? undefined,
      })),
    };
  }

  async function save() {
    const now = new Date();
    meta.value.updatedAt = now;
    await db.update(localSessions).set({
      title: meta.value.title,
      description: meta.value.disposition,
      updatedAt: now.toISOString(),
    }).where(eq(localSessions.id, sessionId.value));
  }

  /** Push a new page to the vignette.
   * @returns an updater that can be called once the AI has finished generating its response.
   */
  async function push({ prompt, system }: { prompt?: string, system?: string }): Promise<PromptUpdater> {
    const ts = new Date().toISOString();
    const sid = sessionId.value;
    const pageIndex = pages.value.length;
    // used for updating current snapshot, if it still exists
    const currSnapId = snapshot.value.id;

    const page: VignettePage = {
      id: crypto.randomUUID(),
      sessionId: sid,
      system,
      prompt,
    };

    await db.transaction(async (tx) => {
      await tx.update(localSessions).set({ updatedAt: ts }).where(eq(localSessions.id, sid));
      await tx.insert(localPages).values({
        ...page,
        createdAt: ts,
      });
    });

    pages.value.push(page);

    return async (response, toolCalls, data) => {
      const _snapshot: Snapshot = {
        id: crypto.randomUUID(),
        sessionId: sid,
        pageIndex,
        data,
        createdAt: new Date(),
      };

      page.response = response;

      if (toolCalls.length) {
        const toolCallsSerialized = page.toolCalls = JSON.stringify(toolCalls);
        await db.transaction(async (tx) => {
          await tx.update(localPages)
            .set({
              response,
              toolCalls: toolCallsSerialized,
            })
            .where(eq(localPages.id, page.id));
          await pushSnapshot(tx, _snapshot);
        });
        snapshot.value = _snapshot;
      } else {
        await db.transaction(async (tx) => {
          await tx.update(localPages)
            .set({ response })
            .where(eq(localPages.id, page.id));
          if (pageIndex % SNAPSHOT_CHECKPOINT_INTERVAL !== 0) {
            await tx.update(localStateSnapshots)
              .set({ pageIndex })
              .where(eq(localStateSnapshots.id, currSnapId));
          } else {
            await pushSnapshot(tx, _snapshot);
            snapshot.value = _snapshot;
          }
        });
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

    const ts = new Date().toISOString();
    const truncateIds = _pages.slice(pageIndex).map(p => p.id);

    // Nuke all pages including the page to fork
    pages.value = _pages = _pages.slice(0, pageIndex);

    await db.transaction(async (tx) => {
      await tx.delete(localPages).where(inArray(localPages.id, truncateIds));
      await tx.delete(localStateSnapshots).where(gte(localStateSnapshots.pageIndex, pageIndex));
      await tx.update(localSessions).set({ updatedAt: ts });
    });

    const _snapshot = await loadSnapshot(pageIndex - 1);
    await replay(_snapshot, _pages);
    snapshot.value = _snapshot;

    // if value === null, clear
    // if value === undefined, use old
    // otherwise, use new
    return await push({
      system: system === null ? undefined : system ?? page.system,
      prompt: prompt === null ? undefined : prompt ?? page.prompt,
    });
  }

  async function pushSnapshot(tx: SQLiteTx, snapshot: Snapshot) {
    // delete any snapshots that aren't on backup intervals
    await tx.delete(localStateSnapshots)
      .where(and(
        eq(localStateSnapshots.sessionId, snapshot.sessionId),
        sql`${localStateSnapshots.pageIndex} % ${SNAPSHOT_CHECKPOINT_INTERVAL} <> 0`,
      ));

    await tx.insert(localStateSnapshots).values({
      id: snapshot.id,
      sessionId: snapshot.sessionId,
      pageIndex: snapshot.pageIndex,
      data: JSON.stringify(snapshot.data),
      createdAt: snapshot.createdAt.toISOString(),
    });
  }

  /** Update only the wording of system prompt, user prompt & AI response. Is NOT intended
   * for receiving an AI response and thus does not return an updater.
   */
  async function update({ pageIndex, system, prompt, response }: UpdateOpts): Promise<void> {
    const page = pages.value[pageIndex];
    if (!page) throw new RangeError(`Page index ${pageIndex} out of bounds`);
    await db.update(localPages)
      .set({
        system: system === null ? null : system ?? page.system,
        prompt: prompt === null ? null : prompt ?? page.prompt,
        response: response === null ? null : response ?? page.response,
      })
      .where(eq(localPages.id, page.id));
  }

  watch(sessionId, async (newId, _oldId, onCleanup) => {
    let mounted = true;

    onCleanup(() => {
      mounted = false;
    });

    try {
      status.value = 'loading';
      const { pages: vignettePages, ...vignetteMeta } = await load(newId);
      const pageIndex = vignettePages.length - 1;
      const _snapshot = await loadSnapshot(pageIndex);
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
      storyId: meta.value.storyId,
      state: snapshot.value.data,
    }),
  };
}
