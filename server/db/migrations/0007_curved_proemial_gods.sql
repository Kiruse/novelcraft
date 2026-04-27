CREATE TABLE "game_session_pages" (
	"id" serial PRIMARY KEY NOT NULL,
	"game_session_id" integer NOT NULL,
	"system" text,
	"prompt" text,
	"response" text,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
ALTER TABLE "game_session_pages" ADD CONSTRAINT "game_session_pages_game_session_id_game_sessions_id_fk" FOREIGN KEY ("game_session_id") REFERENCES "public"."game_sessions"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "game_session_pages_session_idx" ON "game_session_pages" USING btree ("game_session_id","created_at");