import { auth } from '#server/auth/config';
import { db } from '#server/db';
import { user } from '#server/db/schema';
import { eq } from 'drizzle-orm';

export default defineEventHandler(async (event) => {
  const session = await auth.api.getSession({ headers: event.headers });
  if (!session?.user) {
    return { user: null };
  }

  // Fetch full user record including isAuthor
  const [dbUser] = await db
    .select({ isAuthor: user.isAuthor })
    .from(user)
    .where(eq(user.id, session.user.id))
    .limit(1);

  return {
    user: {
      id: session.user.id,
      name: session.user.name,
      email: session.user.email,
      emailVerified: session.user.emailVerified,
      image: session.user.image ?? null,
      isAuthor: dbUser?.isAuthor ?? false,
    },
  };
});
