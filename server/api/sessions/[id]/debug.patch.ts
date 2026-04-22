import { db } from '#server/db';
import { gameSessions, moduleRuntime } from '#server/db/schema/app';
import { eq, and } from 'drizzle-orm';
import { z } from 'zod';

const bodySchema = z.object({
  moduleId: z.string(),
  patch: z.record(z.string(), z.unknown()),
});

export default defineEventHandler(async (event) => {
  const sessionId = getRouterParam(event, 'id');
  if (!sessionId) {
    throw createError({ statusCode: 400, statusMessage: 'Session ID required' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid body: moduleId and patch required' });
  }

  const { moduleId, patch } = parsed.data;

  // Verify session exists
  const session = await db.query.gameSessions.findFirst({
    where: eq(gameSessions.id, parseInt(sessionId)),
  });
  if (!session) {
    throw createError({ statusCode: 404, statusMessage: 'Session not found' });
  }

  // Find existing runtime state
  const existing = await db.query.moduleRuntime.findFirst({
    where: and(
      eq(moduleRuntime.gameSessionId, session.id),
      eq(moduleRuntime.moduleId, moduleId),
    ),
  });

  if (!existing) {
    throw createError({ statusCode: 404, statusMessage: `Module runtime "${moduleId}" not found` });
  }

  // Deep-merge patch into existing state
  const currentState = (existing.data ?? {}) as Record<string, unknown>;
  const newState = deepMerge(currentState, patch);

  await db
    .update(moduleRuntime)
    .set({ data: newState })
    .where(eq(moduleRuntime.id, existing.id));

  return { moduleId, state: newState };
});

function deepMerge(target: Record<string, unknown>, source: Record<string, unknown>): Record<string, unknown> {
  const result = { ...target };
  for (const [key, value] of Object.entries(source)) {
    if (
      value !== null &&
      typeof value === 'object' &&
      !Array.isArray(value) &&
      typeof result[key] === 'object' &&
      result[key] !== null &&
      !Array.isArray(result[key])
    ) {
      result[key] = deepMerge(result[key] as Record<string, unknown>, value as Record<string, unknown>);
    } else {
      result[key] = value;
    }
  }
  return result;
}
