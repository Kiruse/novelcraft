import { db } from '#server/db';
import { gameSessions, moduleRuntime } from '#server/db/schema/app';
import { eq, and } from 'drizzle-orm';
import { getAllModules, type GameplayModule } from '#server/gameplay/gameplayModule';

export default defineEventHandler(async (event) => {
  const sessionId = getRouterParam(event, 'id');
  if (!sessionId) {
    throw createError({ statusCode: 400, statusMessage: 'Session ID required' });
  }

  const session = await db.query.gameSessions.findFirst({
    where: eq(gameSessions.id, parseInt(sessionId)),
    with: { moduleRuntime: true, story: true },
  });

  if (!session) {
    throw createError({ statusCode: 404, statusMessage: 'Session not found' });
  }

  const registeredModules = getAllModules();

  // Assemble debug view: module type → { config, state, knowledge, schema }
  const modules: Record<string, {
    config: unknown;
    state: unknown;
    knowledge: unknown;
    tools: string[];
  }> = {};

  const storyModules = (session.story.modules ?? {}) as Record<string, unknown>;
  const runtimeStates = new Map(
    session.moduleRuntime.map((r) => [r.moduleId, r.data]),
  );

  for (const [type, config] of Object.entries(storyModules)) {
    const modDef = registeredModules.get(type);
    const state = runtimeStates.get(type) ?? null;

    let knowledge = null;
    const toolNames: string[] = [];

    if (modDef) {
      try {
        const ctx = {
          session: { record: session, modules: [{ config, state }] },
          module: modDef,
          config,
          state: state ?? {},
        };
        knowledge = modDef.getKnowledge(ctx as any);
      } catch {
        knowledge = { error: 'Failed to compute knowledge' };
      }
      toolNames.push(...(modDef.tools?.map((t) => t.name) ?? []));
    }

    modules[type] = { config, state, knowledge, tools: toolNames };
  }

  return {
    sessionId: session.id,
    storyId: session.storyId,
    sessionData: session.data,
    modules,
  };
});
