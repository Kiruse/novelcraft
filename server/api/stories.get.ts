import { db } from "../db";
import { stories } from "../db/schema";
import { sql } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  // Get latest published version of each story (excluding vignettes)
  const allStories = await db.query.stories.findMany({
    where: sql`${stories.version} > 0`,
    orderBy: (stories, { desc }) => [desc(stories.createdAt)],
    with: {
      author: {
        columns: {
          id: true,
          name: true,
        },
      },
    },
  });

  // Deduplicate: keep only the latest version per (authorId, storyId)
  const seen = new Set<string>();
  const latest = allStories.filter((s) => {
    const key = `${s.authorId}/${s.storyId}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });

  return { stories: latest };
});
