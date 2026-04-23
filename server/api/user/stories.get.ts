import { db } from "#server/db";
import { stories } from "#server/db/schema";
import { auth } from "#server/auth/config";
import { eq, desc } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const session = await auth.api.getSession({ headers: event.headers });
  if (!session?.user) {
    return { stories: [] };
  }

  const authorStories = await db.query.stories.findMany({
    where: eq(stories.authorId, session.user.id),
    orderBy: desc(stories.updatedAt),
    columns: {
      id: true,
      storyId: true,
      title: true,
      version: true,
      genre: true,
      coverArt: true,
    },
  });

  // Deduplicate: keep only latest version per storyId
  const seen = new Set<string>();
  const deduped = authorStories.filter((s) => {
    const key = s.storyId;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });

  return { stories: deduped };
});
