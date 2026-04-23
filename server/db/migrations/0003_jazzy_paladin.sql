DROP INDEX "stories_author_story_version_idx";--> statement-breakpoint
CREATE UNIQUE INDEX "stories_author_story_version_idx" ON "stories" USING btree ("author_id","story_id","version");