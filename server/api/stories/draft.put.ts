import { db } from '#server/db';
import { stories } from '#server/db/schema/app';
import { user } from '#server/db/schema';
import { auth } from '#server/auth/config';
import { eq, and } from 'drizzle-orm';
import { z } from 'zod';

const bodySchema = z.object({
  storyId: z.string().min(1),
  title: z.string().max(200).optional(),
  description: z.string().max(2000).optional(),
  coverArt: z.url().optional(),
  genre: z.string().max(100).optional(),
  modules: z.record(z.string(), z.unknown()).default({}),
});

export default defineEventHandler(async (event) => {
  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const [dbUser] = await db.select({ isAuthor: user.isAuthor }).from(user).where(eq(user.id, authSession.user.id)).limit(1);
  if (!dbUser?.isAuthor) {
    throw createError({ statusCode: 403, statusMessage: 'Author access required' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({
      statusCode: 400,
      statusMessage: parsed.error.issues.map((i) => i.message).join('; '),
    });
  }

  const { storyId, title, description, coverArt, genre, modules } = parsed.data;

  // Find existing draft for this author
  const existing = await db.query.stories.findFirst({
    where: and(
      eq(stories.authorId, authSession.user.id),
      eq(stories.storyId, storyId),
      eq(stories.version, 0),
    ),
  });

  if (existing) {
    // Story ID must never change on an existing draft
    if (existing.storyId !== storyId) {
      throw createError({
        statusCode: 403,
        statusMessage: 'Story ID cannot be changed',
      });
    }

    const [updated] = await db
      .update(stories)
      .set({
        title: title ?? existing.title,
        description: description ?? null,
        coverArt: coverArt ?? null,
        genre: genre ?? null,
        modules: modules,
      })
      .where(eq(stories.id, existing.id))
      .returning();

    return { story: { ...updated, authorName: authSession.user.name } };
  }

  // New draft — storyId is the only required field
  if (!title) {
    throw createError({
      statusCode: 400,
      statusMessage: 'Title is required for new drafts',
    });
  }

  const [story] = await db
    .insert(stories)
    .values({
      storyId,
      authorId: authSession.user.id,
      version: 0,
      title,
      description: description ?? null,
      coverArt: coverArt ?? null,
      genre: genre ?? null,
      modules: modules,
    })
    .returning();

  return { story: { ...story, authorName: authSession.user.name } };
});
