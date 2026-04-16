import { db } from '#server/db';
import { gameSessions } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq } from 'drizzle-orm';
import { promptGameAgent } from '#server/gameplay/agentLoop';
import { z } from 'zod';

const bodySchema = z.object({
  content: z.string().min(1).max(4000),
});

export default defineEventHandler(async (event) => {
  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const sessionId = getRouterParam(event, 'id');
  if (!sessionId) {
    throw createError({ statusCode: 400, statusMessage: 'Session ID is required' });
  }

  // Verify the session belongs to this user
  const session = await db.query.gameSessions.findFirst({
    where: eq(gameSessions.id, parseInt(sessionId)),
  });

  if (!session) {
    throw createError({ statusCode: 404, statusMessage: 'Session not found' });
  }

  if (session.playerId !== authSession.user.id) {
    throw createError({ statusCode: 403, statusMessage: 'Not your session' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid message content' });
  }

  const response = await promptGameAgent(session.id, parsed.data.content);

  return { response };
});
