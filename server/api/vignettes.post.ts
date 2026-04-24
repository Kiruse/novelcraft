import { db } from '#server/db';
import { stories } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { z } from 'zod';
import { randomUUID } from 'node:crypto';

const bodySchema = z.object({
  disposition: z.string().max(4000).optional().default(''),
});

export default defineEventHandler(async (event) => {
  const session = await auth.api.getSession({ headers: event.headers });
  if (!session?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid input' });
  }

  const storyId = `vignette:${randomUUID()}`;

  const rows = await db
    .insert(stories)
    .values({
      storyId,
      authorId: session.user.id,
      version: 0,
      title: 'Untitled Vignette',
      description: parsed.data.disposition || null,
      isVignette: true,
      modules: {},
    })
    .returning();

  return { vignette: rows[0] };
});
