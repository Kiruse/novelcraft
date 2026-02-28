import { db } from "../db";
import { gameSessions } from "../db/schema";
import { eq, desc } from "drizzle-orm";
import { auth } from "../auth/config";

export default defineEventHandler(async (event) => {
  const session = await auth.api.getSession({
    headers: event.headers,
  });

  if (!session?.user) {
    return { sessions: [] };
  }

  const userSessions = await db.query.gameSessions.findMany({
    where: eq(gameSessions.playerId, session.user.id),
    orderBy: (gameSessions, { desc }) => [desc(gameSessions.updatedAt)],
    with: {
      story: {
        columns: {
          id: true,
          title: true,
          coverArt: true,
          genre: true,
        },
      },
    },
    limit: 5,
  });

  return { sessions: userSessions };
});
