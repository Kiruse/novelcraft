/**
 * Incremental pseudo-XML parser for story suggestions.
 *
 * Tolerant of whitespace inside tags: `< title >`, `</ title >`, etc.
 *
 * Emits events as close to generation time as possible:
 * - `onOpen` fires on `<suggestion>` — card appears immediately
 * - `onField` fires on every chunk with the FULL accumulated value so far
 * - `onComplete` fires on `</suggestion>` — card is done
 */

export interface ParsedSuggestion {
  storyId: string;
  title: string;
  genre: string;
  description: string;
}

type TagName = 'storyId' | 'title' | 'genre' | 'description';

const KNOWN_TAGS: ReadonlySet<string> = new Set(['storyId', 'title', 'genre', 'description']);

/** Match an opening tag like `< tagName >`, capturing the tag name. */
const OPEN_TAG_RE = /^<\s*(\w+)\s*>/;
/** Match a closing tag like `</ tagName >`, capturing the tag name. */
const CLOSE_TAG_RE = /^<\/\s*(\w+)\s*>/;

export class SuggestionParser {
  private buf = '';
  private current: Partial<ParsedSuggestion> | null = null;
  private activeTag: TagName | null = null;
  /** Accumulated content for the currently active tag. */
  private activeAccumulated = '';
  private index = 0;

  readonly onOpen: (index: number) => void;
  readonly onField: (index: number, field: TagName, value: string) => void;
  readonly onComplete: (index: number, suggestion: ParsedSuggestion) => void;

  constructor(callbacks: {
    onOpen: (index: number) => void;
    onField: (index: number, field: TagName, value: string) => void;
    onComplete: (index: number, suggestion: ParsedSuggestion) => void;
  }) {
    this.onOpen = callbacks.onOpen;
    this.onField = callbacks.onField;
    this.onComplete = callbacks.onComplete;
  }

  push(text: string): void {
    this.buf += text;
    this.drain();
  }

  finish(): void {
    this.drain();
  }

  private emitField(value: string): void {
    if (this.current && this.activeTag) {
      (this.current as Record<string, string>)[this.activeTag] = value;
      this.onField(this.index, this.activeTag, value);
    }
  }

  private closeActiveTag(): void {
    if (!this.activeTag) return;
    // Emit the final accumulated value (trimmed)
    this.emitField(this.activeAccumulated.trim());
    this.activeTag = null;
    this.activeAccumulated = '';
  }

  /**
   * Look for a close tag for `tagName` (tolerant of whitespace) in the buffer.
   * Returns the index where the content ends and the full match length.
   */
  private findCloseTag(tagName: string): { contentEnd: number; tagLen: number } | null {
    const re = new RegExp(`<\\/\\s*${tagName}\\s*>`);
    const m = this.buf.match(re);
    if (!m) return null;
    return { contentEnd: m.index!, tagLen: m[0].length };
  }

  private drain(): void {
    while (this.buf.length > 0) {
      // --- Inside a content tag: accumulate, look for close ---
      if (this.activeTag) {
        const close = this.findCloseTag(this.activeTag);

        if (close) {
          // Close tag found — add remaining buffer content, emit final value
          this.activeAccumulated += this.buf.slice(0, close.contentEnd);
          this.buf = this.buf.slice(close.contentEnd + close.tagLen);
          this.closeActiveTag();
          continue;
        }

        // No close tag yet — absorb a safe prefix into the accumulator.
        // Keep a tail because the close tag might be split across chunks.
        const tailGuard = this.activeTag.length + 6;
        const safeEnd = this.buf.length - tailGuard;
        if (safeEnd > 0) {
          this.activeAccumulated += this.buf.slice(0, safeEnd);
          this.buf = this.buf.slice(safeEnd);
          this.emitField(this.activeAccumulated);
        }
        return;
      }

      // --- Outside a tag: skip to next '<' ---
      const lt = this.buf.indexOf('<');
      if (lt === -1) { this.buf = ''; return; }
      if (lt > 0) this.buf = this.buf.slice(lt);

      // Try closing tag first (e.g. </ suggestion >)
      const closeMatch = this.buf.match(CLOSE_TAG_RE);
      if (closeMatch && closeMatch[1] && closeMatch[1].trim().toLowerCase() === 'suggestion') {
        if (this.current && this.current.storyId && this.current.title && this.current.genre && this.current.description) {
          this.onComplete(this.index, this.current as ParsedSuggestion);
        }
        this.current = null;
        this.index++;
        this.buf = this.buf.slice(closeMatch[0].length);
        continue;
      }

      // Try opening < suggestion >
      const openMatch = this.buf.match(OPEN_TAG_RE);
      if (openMatch && openMatch[1] && openMatch[1].trim().toLowerCase() === 'suggestion') {
        this.current = {};
        this.onOpen(this.index);
        this.buf = this.buf.slice(openMatch[0].length);
        continue;
      }

      // Known content tag
      if (openMatch && openMatch[1] && KNOWN_TAGS.has(openMatch[1].trim())) {
        this.activeTag = openMatch[1].trim() as TagName;
        this.activeAccumulated = '';
        this.buf = this.buf.slice(openMatch[0].length);
        continue;
      }

      // Unknown or incomplete tag — skip past '>' if we have one
      const gt = this.buf.indexOf('>', 1);
      if (gt === -1) return;
      this.buf = this.buf.slice(gt + 1);
    }
  }
}
