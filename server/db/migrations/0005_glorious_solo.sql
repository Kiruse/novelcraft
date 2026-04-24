CREATE TYPE "public"."vignette_status" AS ENUM('draft', 'playing', 'completed', 'abandoned');--> statement-breakpoint
CREATE TABLE "vignette_messages" (
	"id" serial PRIMARY KEY NOT NULL,
	"vignette_id" integer NOT NULL,
	"role" "message_role" NOT NULL,
	"contents" text NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "vignettes" (
	"id" serial PRIMARY KEY NOT NULL,
	"player_id" text NOT NULL,
	"disposition" text DEFAULT '' NOT NULL,
	"title" text,
	"premise" text,
	"status" "vignette_status" DEFAULT 'draft' NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "vignette_messages" ADD CONSTRAINT "vignette_messages_vignette_id_vignettes_id_fk" FOREIGN KEY ("vignette_id") REFERENCES "public"."vignettes"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "vignettes" ADD CONSTRAINT "vignettes_player_id_user_id_fk" FOREIGN KEY ("player_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "vignette_messages_vignette_created_idx" ON "vignette_messages" USING btree ("vignette_id","created_at");--> statement-breakpoint
CREATE INDEX "vignettes_player_updated_idx" ON "vignettes" USING btree ("player_id","updated_at");