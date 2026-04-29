DROP TABLE "game_session_messages" CASCADE;--> statement-breakpoint
DROP TABLE "game_session_pages" CASCADE;--> statement-breakpoint
DROP TABLE "game_sessions" CASCADE;--> statement-breakpoint
DROP TABLE "module_runtime" CASCADE;--> statement-breakpoint
ALTER TABLE "stories" DROP COLUMN "is_vignette";--> statement-breakpoint
DROP TYPE "public"."message_role";