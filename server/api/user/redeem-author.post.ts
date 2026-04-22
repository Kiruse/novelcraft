import { auth } from '#server/auth/config';
import { db } from '#server/db';
import { user } from '#server/db/schema';
import { eq } from 'drizzle-orm';
import { z } from 'zod';

const bodySchema = z.object({
  code: z.string().min(1),
});

export default defineEventHandler(async (event) => {
  const session = await auth.api.getSession({ headers: event.headers });
  if (!session?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: 'Code is required' });
  }

  const validCodes = (process.env.AUTHOR_REGISTRATION_CODES ?? '')
    .split(',')
    .map((c) => c.trim())
    .filter(Boolean);

  if (!validCodes.includes(parsed.data.code)) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid registration code' });
  }

  const [updated] = await db
    .update(user)
    .set({ isAuthor: true })
    .where(eq(user.id, session.user.id))
    .returning();

  return { isAuthor: updated?.isAuthor ?? false };
});
