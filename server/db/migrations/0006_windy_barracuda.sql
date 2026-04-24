DROP TABLE "vignette_messages" CASCADE;--> statement-breakpoint
DROP TABLE "vignettes" CASCADE;--> statement-breakpoint
ALTER TABLE "stories" ADD COLUMN "is_vignette" boolean DEFAULT false NOT NULL;--> statement-breakpoint
DROP TYPE "public"."vignette_status";