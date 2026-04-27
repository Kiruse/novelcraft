import { DEFAULT_MODEL } from '#shared/prompts';
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

export interface StreamLlmOptions {
  persona: string;
  messages: LlmMessage[];
}

// --- Functions ---

/** Build conversation messages from pages + vignette context. */
export function buildMessages(opts: BuildMessagesOpts): LlmMessage[] {
  const context = opts.description
    ? `Title: ${opts.title}\nPremise: ${opts.description}`
    : `Title: ${opts.title}`;

  const msgs: LlmMessage[] = [
    { author: 'user', content: `[Context] ${context}` },
    { author: 'ai', content: '(Vignette started)' },
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

/**
 * Call /api/llm/prompt and stream the response as an async generator.
 * Yields each text chunk as it arrives.
 */
export async function* streamLlm({
  persona,
  messages,
}: StreamLlmOptions): AsyncGenerator<string> {
  const response = await fetch('/api/llm/prompt', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: DEFAULT_MODEL,
      messages,
      persona,
    }),
  });

  if (!response.ok) throw new Error(`LLM request failed: HTTP ${response.status}`);

  const reader = response.body?.getReader();
  if (!reader) throw new Error('No response body');

  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });

    const parts = buffer.split('\n\n');
    buffer = parts.pop()!;

    for (const part of parts) {
      const lines = part.split('\n');
      const eventType = lines.find(l => l.startsWith('event: '))?.slice(7).trim();
      const dataLine = lines.find(l => l.startsWith('data: '))?.slice(6) ?? '';
      let data = '';
      if (dataLine) {
        try { data = JSON.parse(dataLine) as string; } catch { data = dataLine; }
      }

      if (eventType === 'text') {
        yield data;
      }
    }
  }
}
