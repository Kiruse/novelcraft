import { db } from '#server/db';
import { stories } from '#server/db/schema/app';
import { user } from '#server/db/schema';
import { auth } from '#server/auth/config';
import { eq, and, sql } from 'drizzle-orm';

export default defineEventHandler(async (event) => {
  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const [dbUser] = await db.select({ isAuthor: user.isAuthor }).from(user).where(eq(user.id, authSession.user.id)).limit(1);
  if (!dbUser?.isAuthor) {
    throw createError({ statusCode: 403, statusMessage: 'Author access required' });
  }

  const author = getRouterParam(event, 'author');
  const storyId = getRouterParam(event, 'id');

  if (!author || !storyId) {
    throw createError({ statusCode: 400, statusMessage: 'Author and story ID are required' });
  }

  // Verify the logged-in user matches the author param (case-insensitive)
  if (authSession.user.name.toLowerCase() !== author.toLowerCase()) {
    throw createError({ statusCode: 403, statusMessage: 'Not your story' });
  }

  // Find existing version 0 draft
  const existingDraft = await db.query.stories.findFirst({
    where: and(
      eq(stories.authorId, authSession.user.id),
      eq(stories.storyId, storyId),
      eq(stories.version, 0),
    ),
  });

  if (existingDraft) {
    return { story: existingDraft };
  }

  // No draft — find the latest version (published or not) to pre-populate from
  const latest = await db.query.stories.findFirst({
    where: and(
      eq(stories.authorId, authSession.user.id),
      eq(stories.storyId, storyId),
    ),
    orderBy: sql`${stories.version} DESC`,
  });

  if (!latest) {
    // No story exists at all — return a blank template for the builder
    return {
      story: {
        id: 0,
        storyId,
        authorId: authSession.user.id,
        version: 0,
        title: '',
        description: null,
        coverArt: null,
        genre: null,
        modules: {},
      },
    };
  }

  // Create version 0 draft pre-populated from source
  const [draft] = await db
    .insert(stories)
    .values({
      storyId: latest.storyId,
      authorId: authSession.user.id,
      version: 0,
      title: latest.title,
      description: latest.description,
      coverArt: latest.coverArt,
      genre: latest.genre,
      modules: latest.modules,
    })
    .returning();

  return { story: draft };
});
