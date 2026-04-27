import { db } from '#server/db';
import { gameSessionPages, gameSessions, stories } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq, and } from 'drizzle-orm';
import { z } from 'zod';

const bodySchema = z.object({
  response: z.string().max(50000).optional(),
  prompt: z.string().max(4000).optional(),
  system: z.string().max(4000).nullable().optional(),
});

export default defineEventHandler(async (event) => {
  const session = await auth.api.getSession({ headers: event.headers });
  if (!session?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  // Verify the vignette story
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

  // Load the page
  const pageId = getRouterParam(event, 'pageId');
  if (!pageId) {
    throw createError({ statusCode: 400, statusMessage: 'Page ID is required' });
  }

  const page = await db.query.gameSessionPages.findFirst({
    where: eq(gameSessionPages.id, parseInt(pageId)),
    with: { gameSession: true },
  });

  if (!page) {
    throw createError({ statusCode: 404, statusMessage: 'Page not found' });
  }

  // Enforce the page belongs to this vignette
  if (page.gameSession.storyId !== story.id) {
    throw createError({ statusCode: 403, statusMessage: 'Page does not belong to this vignette' });
  }

  if (page.gameSession.playerId !== session.user.id) {
    throw createError({ statusCode: 403, statusMessage: 'Not your page' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid input' });
  }

  const updates: Record<string, string | null> = {};
  if (parsed.data.response !== undefined) updates.response = parsed.data.response;
  if (parsed.data.prompt !== undefined) updates.prompt = parsed.data.prompt;
  if (parsed.data.system !== undefined) updates.system = parsed.data.system;

  if (Object.keys(updates).length === 0) {
    return { page };
  }

  const rows = await db
    .update(gameSessionPages)
    .set(updates)
    .where(eq(gameSessionPages.id, parseInt(pageId)))
    .returning();

  return { page: rows[0] };
});
