CREATE TABLE `local_lore_entries` (
	`id` text PRIMARY KEY NOT NULL,
	`story_id` text NOT NULL,
	`title` text NOT NULL,
	`content` text NOT NULL,
	`tags` text,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `local_onboarding` (
	`completed` integer DEFAULT 0 NOT NULL
);
--> statement-breakpoint
CREATE TABLE `local_pages` (
	`id` text PRIMARY KEY NOT NULL,
	`session_id` text NOT NULL,
	`system` text,
	`prompt` text,
	`response` text,
	`tool_calls` text,
	`created_at` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `local_profiles` (
	`id` text PRIMARY KEY NOT NULL,
	`name` text NOT NULL,
	`fields` text NOT NULL,
	`active` integer DEFAULT 0 NOT NULL,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `local_sessions` (
	`id` text PRIMARY KEY NOT NULL,
	`story_id` text NOT NULL,
	`title` text NOT NULL,
	`description` text,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `local_state_snapshots` (
	`id` text PRIMARY KEY NOT NULL,
	`session_id` text NOT NULL,
	`page_index` integer NOT NULL,
	`data` text NOT NULL,
	`created_at` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `local_stories` (
	`id` text PRIMARY KEY NOT NULL,
	`title` text NOT NULL,
	`description` text,
	`config` text NOT NULL,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL
);
