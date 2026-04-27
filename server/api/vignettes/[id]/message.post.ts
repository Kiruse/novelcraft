import { db } from '#server/db';
import { stories, gameSessions, gameSessionPages, moduleRuntime } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq, and } from 'drizzle-orm';
import { z } from 'zod';

const bodySchema = z.object({
  content: z.string().max(4000).nullable().optional(),
  system: z.string().max(4000).optional(),
  /** Omit to start a new session; provide to append to an existing one. */
  sessionId: z.number().int().positive().optional(),
});

export default defineEventHandler(async (event) => {
  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const id = getRouterParam(event, 'id');
  if (!id) {
    throw createError({ statusCode: 400, statusMessage: 'Vignette ID is required' });
  }

  const story = await db.query.stories.findFirst({
    where: and(
      eq(stories.id, parseInt(id)),
      eq(stories.isVignette, true),
    ),
  });

  if (!story) {
    throw createError({ statusCode: 404, statusMessage: 'Vignette not found' });
  }
  if (story.authorId !== authSession.user.id) {
    throw createError({ statusCode: 403, statusMessage: 'Not your vignette' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid input' });
  }

  let sessionId: number;

  if (parsed.data.sessionId) {
    // Existing session — verify ownership
    const gameSession = await db.query.gameSessions.findFirst({
      where: and(
        eq(gameSessions.id, parsed.data.sessionId),
        eq(gameSessions.storyId, story.id),
        eq(gameSessions.playerId, authSession.user.id),
      ),
    });

    if (!gameSession) {
      throw createError({ statusCode: 404, statusMessage: 'Session not found' });
    }

    sessionId = gameSession.id;
  } else {
    // No session — create one
    const existingSession = await db.query.gameSessions.findFirst({
      where: and(
        eq(gameSessions.storyId, story.id),
        eq(gameSessions.playerId, authSession.user.id),
      ),
    });

    if (existingSession) {
      throw createError({ statusCode: 400, statusMessage: 'Vignette already started' });
    }

    const sessionRows = await db
      .insert(gameSessions)
      .values({
        playerId: authSession.user.id,
        storyId: story.id,
        data: {},
      })
      .returning();
    sessionId = sessionRows[0]!.id;

    await db.insert(moduleRuntime).values({
      gameSessionId: sessionId,
      moduleId: 'system_prompt',
      data: { version: 1 },
    });
  }

  // Create a new page (response to be filled after LLM generation)
  const pageRows = await db.insert(gameSessionPages).values({
    gameSessionId: sessionId,
    system: parsed.data.system ?? null,
    prompt: parsed.data.content ?? null,
    response: null,
  }).returning();
  const page = pageRows[0]!;

  return { sessionId, pageId: page.id };
});
