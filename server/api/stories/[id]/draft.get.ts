import { db } from '#server/db';
import { stories } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq, and, sql } from 'drizzle-orm';

export default defineEventHandler(async (event) => {
  const { public: { storyBuilder } } = useRuntimeConfig();
  if (!storyBuilder) {
    throw createError({ statusCode: 404, statusMessage: 'Not found' });
  }

  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const id = getRouterParam(event, 'id');
  if (!id) {
    throw createError({ statusCode: 400, statusMessage: 'Story ID is required' });
  }

  const storyPk = parseInt(id, 10);

  // Find existing version 0 draft
  const existingDraft = await db.query.stories.findFirst({
    where: and(
      eq(stories.id, storyPk),
      eq(stories.version, 0),
    ),
  });

  if (existingDraft) {
    return { story: existingDraft };
  }

  // No draft — find the story (any version) to get its storyId
  const anyVersion = await db.query.stories.findFirst({
    where: eq(stories.id, storyPk),
  });

  if (!anyVersion) {
    throw createError({ statusCode: 404, statusMessage: 'Story not found' });
  }

  // Find the latest version to pre-populate from
  const latest = await db.query.stories.findFirst({
    where: and(
      eq(stories.storyId, anyVersion.storyId),
      sql`${stories.version} > 0`,
    ),
    orderBy: sql`${stories.version} DESC`,
  });

  const source = latest ?? anyVersion;

  // Create version 0 draft pre-populated from source
  const [draft] = await db
    .insert(stories)
    .values({
      storyId: source.storyId,
      authorId: authSession.user.id,
      version: 0,
      title: source.title,
      description: source.description,
      coverArt: source.coverArt,
      genre: source.genre,
      modules: source.modules,
    })
    .returning();

  return { story: draft };
});
