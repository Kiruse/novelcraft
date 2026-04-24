import { db } from '#server/db';
import { stories } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq, desc, and } from 'drizzle-orm';

export default defineEventHandler(async (event) => {
  const session = await auth.api.getSession({ headers: event.headers });
  if (!session?.user) {
    return { vignettes: [] };
  }

  const userVignettes = await db.query.stories.findMany({
    where: and(
      eq(stories.authorId, session.user.id),
      eq(stories.isVignette, true),
    ),
    orderBy: desc(stories.updatedAt),
    limit: 20,
  });

  return { vignettes: userVignettes };
});
