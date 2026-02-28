import { db } from "../db";
import { stories } from "../db/schema";

export default defineEventHandler(async (event) => {
  const allStories = await db.query.stories.findMany({
    orderBy: (stories, { asc }) => [asc(stories.title)],
    with: {
      author: {
        columns: {
          id: true,
          name: true,
        },
      },
    },
  });

  return { stories: allStories };
});
