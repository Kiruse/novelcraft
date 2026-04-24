import { db } from '#server/db';
import { stories } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq, and } from 'drizzle-orm';
import { z } from 'zod';

const bodySchema = z.object({
  disposition: z.string().max(4000).optional(),
  title: z.string().max(200).optional(),
  premise: z.string().max(4000).optional(),
});

export default defineEventHandler(async (event) => {
  const session = await auth.api.getSession({ headers: event.headers });
  if (!session?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const id = getRouterParam(event, 'id');
  if (!id) {
    throw createError({ statusCode: 400, statusMessage: 'Vignette ID is required' });
  }

  const existing = await db.query.stories.findFirst({
    where: and(
      eq(stories.id, parseInt(id)),
      eq(stories.isVignette, true),
    ),
  });

  if (!existing) {
    throw createError({ statusCode: 404, statusMessage: 'Vignette not found' });
  }
  if (existing.authorId !== session.user.id) {
    throw createError({ statusCode: 403, statusMessage: 'Not your vignette' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid input' });
  }

  const updates: Record<string, unknown> = {};
  if (parsed.data.disposition !== undefined) updates.description = parsed.data.disposition;
  if (parsed.data.title !== undefined) updates.title = parsed.data.title;

  if (Object.keys(updates).length === 0) {
    return { vignette: existing };
  }

  const rows = await db
    .update(stories)
    .set(updates)
    .where(eq(stories.id, parseInt(id)))
    .returning();

  return { vignette: rows[0] };
});
