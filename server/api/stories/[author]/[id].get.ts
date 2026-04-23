import { db } from "#server/db";
import { stories } from "#server/db/schema/app";
import { user } from "#server/db/schema";
import { auth } from "#server/auth/config";
import { eq, and, sql } from "drizzle-orm";

export default defineEventHandler(async (event) => {
  const author = getRouterParam(event, "author");
  const storyId = getRouterParam(event, "id");

  if (!author || !storyId) {
    throw createError({
      statusCode: 400,
      statusMessage: "Author and story ID are required",
    });
  }

  const query = getQuery(event);
  const isTest = query.test === "1";

  // Resolve author name to user id (case-insensitive)
  const [authorUser] = await db
    .select({ id: user.id })
    .from(user)
    .where(sql`LOWER(${user.name}) = LOWER(${author})`)
    .limit(1);

  if (!authorUser) {
    throw createError({ statusCode: 404, statusMessage: "Author not found" });
  }

  // Check auth upfront — needed for author-only access below
  const session = await auth.api.getSession({ headers: event.headers });
  const isAuthor = session?.user?.id === authorUser.id;

  // Test mode: only the author can view drafts
  if (isTest && !isAuthor) {
    throw createError({ statusCode: 403, statusMessage: "Only the author can preview drafts" });
  }

  // Get latest version for this author + storyId
  const story = await db.query.stories.findFirst({
    where: and(
      eq(stories.authorId, authorUser.id),
      eq(stories.storyId, storyId),
      sql`${stories.version} >= 0`,
    ),
    orderBy: sql`${stories.version} DESC`,
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
    throw createError({ statusCode: 404, statusMessage: "Story not found" });
  }

  // Drafts are only visible to the author
  if (story.version === 0 && !isAuthor) {
    throw createError({ statusCode: 404, statusMessage: "Story not found" });
  }

  return { story };
});
