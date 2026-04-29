import type { GamePage } from '~/utils/msgUtils';

// --- Types ---

export interface LlmMessage {
  author: string;
  content: string;
}

export interface BuildMessagesOpts {
  /** Story title for context. */
  title: string;
  /** Story description/premise for context. */
  description: string | null | undefined;
  /** Current pages to build conversation from. */
  pages: GamePage[];
  /** Override the last page's prompt/response (e.g. suppress response for generation). */
  lastPageOverride?: { prompt?: string | null; response?: string | null };
}

// --- Functions ---

/** Build conversation messages from pages + context. */
export function buildMessages(opts: BuildMessagesOpts): LlmMessage[] {
  const context = opts.description
    ? `Title: ${opts.title}\nPremise: ${opts.description}`
    : `Title: ${opts.title}`;

  const msgs: LlmMessage[] = [
    { author: 'user', content: `[Context] ${context}` },
    { author: 'ai', content: '(Story started)' },
  ];

  for (let i = 0; i < opts.pages.length; i++) {
    const isLast = i === opts.pages.length - 1;
    const page = opts.pages[i]!;

    // Inject the page's system field as a system message
    if (page.system) {
      msgs.push({ author: 'system', content: page.system });
    }

    const effectivePrompt = (isLast && opts.lastPageOverride?.prompt !== undefined)
      ? opts.lastPageOverride.prompt
      : page.prompt;
    const effectiveResponse = (isLast && opts.lastPageOverride?.response !== undefined)
      ? opts.lastPageOverride.response
      : page.response;

    if (effectivePrompt) {
      msgs.push({ author: 'user', content: effectivePrompt });
    }
    if (effectiveResponse) {
      msgs.push({ author: 'ai', content: effectiveResponse });
    }
  }

  return msgs;
}
