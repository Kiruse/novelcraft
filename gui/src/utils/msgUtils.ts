import { marked } from 'marked';

/** Replace em dashes (and surrounding whitespace) with ` - `. */
export function normalizeContent(text: string): string {
  return text.replace(/\s*—\s*/g, ' - ');
}

/** Render markdown text to an HTML string. */
export function renderMarkdown(text: string): string {
  return marked.parse(normalizeContent(text), { async: false }) as string;
}

export const getErrorDisplay = (err: any) =>
  err instanceof Error
    ? err.message
    : err + '';
