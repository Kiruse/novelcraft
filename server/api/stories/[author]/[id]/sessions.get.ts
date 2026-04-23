import { db } from "#server/db";
import { gameSessions, stories } from "#server/db/schema/app";
import { user } from "#server/db/schema";
import { auth } from "#server/auth/config";
import { eq, and, desc, sql } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    return { sessions: [] };
  }

  const author = getRouterParam(event, "author");
  const storyId = getRouterParam(event, "id");
  if (!author || !storyId) {
    throw createError({ statusCode: 400, statusMessage: "Author and story ID are required" });
  }

  // Resolve author name to user id (case-insensitive)
  const [authorUser] = await db
    .select({ id: user.id })
    .from(user)
    .where(sql`LOWER(${user.name}) = LOWER(${author})`)
    .limit(1);

  if (!authorUser) {
    return { sessions: [] };
  }

  // Find any version of this story to get its serial PK
  const story = await db.query.stories.findFirst({
    where: and(
      eq(stories.authorId, authorUser.id),
      eq(stories.storyId, storyId),
    ),
    columns: { id: true },
  });

  if (!story) {
    return { sessions: [] };
  }

  const userSessions = await db.query.gameSessions.findMany({
    where: and(
      eq(gameSessions.playerId, authSession.user.id),
      eq(gameSessions.storyId, story.id),
    ),
    with: {
      story: {
        columns: {
          version: true,
        },
      },
    },
    orderBy: desc(gameSessions.updatedAt),
    limit: 20,
  });

  return { sessions: userSessions };
});
