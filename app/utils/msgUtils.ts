import { marked } from 'marked';

/** A page in the game session. */
export interface GamePage {
  id: number | string;
  /** System instructions for this page. */
  system: string | null;
  /** The user's prompt. */
  prompt: string | null;
  /** The agent's response (markdown source). */
  response: string | null;
}

/** Raw page shape coming from the API. */
export interface RawPage {
  id: number;
  gameSessionId: number;
  system: string | null;
  prompt: string | null;
  response: string | null;
  createdAt: string;
}

/** Replace em dashes (and surrounding whitespace) with ` - `. */
export function normalizeContent(text: string): string {
  return text.replace(/\s*—\s*/g, ' - ');
}

/** Render markdown text to an HTML string. */
export function renderMarkdown(text: string): string {
  return marked.parse(normalizeContent(text), { async: false }) as string;
}
