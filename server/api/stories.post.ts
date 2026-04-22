import { db } from '#server/db';
import { stories } from '#server/db/schema';
import { user } from '#server/db/schema';
import { auth } from '#server/auth/config';
import { eq } from 'drizzle-orm';
import { z } from 'zod';

const bodySchema = z.object({
  storyId: z.string().min(1),
  title: z.string().min(1).max(200),
  genre: z.string().min(1).max(100),
  description: z.string().min(100, 'Description must be at least 100 characters').max(2000),
  coverArt: z.string().url().optional(),
  modules: z.record(z.string(), z.unknown()).default({}),
});

export default defineEventHandler(async (event) => {
  const { public: { storyBuilder } } = useRuntimeConfig();
  if (!storyBuilder) {
    throw createError({ statusCode: 404, statusMessage: 'Not found' });
  }

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

  const [story] = await db
    .insert(stories)
    .values({
      storyId,
      authorId: authSession.user.id,
      version: 1,
      title,
      description,
      coverArt: coverArt ?? null,
      genre,
      modules: modules,
    })
    .returning();

  return { story };
});
