import { db } from "../../db";
import { stories } from "../../db/schema";
import { eq } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const storyId = getRouterParam(event, "id");

  if (!storyId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Story ID is required",
    });
  }

  const story = await db.query.stories.findFirst({
    where: eq(stories.id, parseInt(storyId)),
    with: {
      author: {
        columns: {
          id: true,
          name: true,
          image: true,
        },
      },
    },
  });

  if (!story) {
    throw createError({
      statusCode: 404,
      statusMessage: "Story not found",
    });
  }

  return { story };
});
