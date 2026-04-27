import { db } from '#server/db';
import { stories, gameSessions } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq, and, desc } from 'drizzle-orm';

export default defineEventHandler(async (event) => {
  const session = await auth.api.getSession({ headers: event.headers });
  if (!session?.user) {
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
  if (story.authorId !== session.user.id) {
    throw createError({ statusCode: 403, statusMessage: 'Not your vignette' });
  }

  // Load latest game session (if any) with its pages
  const gameSession = await db.query.gameSessions.findFirst({
    where: eq(gameSessions.storyId, story.id),
    orderBy: desc(gameSessions.updatedAt),
    with: {
      pages: {
        orderBy: (p, { asc }) => [asc(p.createdAt)],
      },
    },
  });

  return {
    vignette: story,
    session: gameSession ?? null,
    pages: gameSession?.pages ?? [],
  };
});
