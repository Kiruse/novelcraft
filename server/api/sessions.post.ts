import { db } from '#server/db';
import { gameSessions, moduleRuntime, stories } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq } from 'drizzle-orm';
import { z } from 'zod';
import { getAllModules } from '#server/gameplay/gameplayModule';

const bodySchema = z.object({
  storyId: z.number().int().positive(),
});

export default defineEventHandler(async (event) => {
  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid storyId' });
  }

  const { storyId } = parsed.data;

  // Load the story to get module configs
  const story = await db.query.stories.findFirst({
    where: eq(stories.id, storyId),
  });

  if (!story) {
    throw createError({ statusCode: 404, statusMessage: 'Story not found' });
  }

  // Create the game session
  const rows = await db
    .insert(gameSessions)
    .values({
      playerId: authSession.user.id,
      storyId,
      data: {},
    })
    .returning();
  const session = rows[0]!;

  // Initialize module runtime states from story config
  const storyModules = (story.modules ?? {}) as unknown as Record<string, unknown>;
  const registeredModules = getAllModules();

  for (const [moduleType, config] of Object.entries(storyModules)) {
    const modDef = registeredModules.get(moduleType);
    if (!modDef) continue;

    const initialState = initializeModuleState(modDef, config);
    await db.insert(moduleRuntime).values({
      gameSessionId: session.id,
      moduleId: moduleType,
      data: initialState,
    });
  }

  return { session };
});

/** Derive initial runtime state from a module's config using its state schema defaults. */
function initializeModuleState(modDef: { state: { parse: (v: unknown) => Record<string, unknown> } }, config: unknown): Record<string, unknown> {
  // Let each module's state schema parse a sensible default. Modules that need
  // config-derived init (e.g. starting location from map config) can be handled
  // here with module-specific logic.
  // For now, we try to parse an empty object and fall back to null.
  try {
    return modDef.state.parse({});
  } catch {
    return {};
  }
}
