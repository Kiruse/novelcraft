import type { GamePage } from '~/utils/msgUtils';

export interface Context {
  [key: string]: string | Context;
}

export interface LlmMessage {
  author: string;
  content: string;
}

export interface BuildMessagesOpts {
  title: string;
  description: string | null | undefined;
  pages: GamePage[];
  pageIndex?: number;
  lastPageOverride?: { prompt?: string | null; response?: string | null };
}

export interface BuildMessagesResult {
  context: Context;
  messages: LlmMessage[];
}

export function buildMessages({
  title,
  description = undefined,
  pages,
  pageIndex = pages.length - 1,
  lastPageOverride,
}: BuildMessagesOpts): BuildMessagesResult {
  const context: any = {
    story: {
      title,
    },
  };

  if (description) context.story.description = description;

  pages = pages.slice(0, pageIndex + 1);

  const messages: LlmMessage[] = [];

  for (let i = 0; i < pages.length; i++) {
    const isLast = i === pages.length - 1;
    const page = pages[i]!;

    // Inject the page's system field as a system message
    if (page.system) {
      messages.push({ author: 'system', content: page.system });
    }

    const effectivePrompt = (isLast && lastPageOverride?.prompt !== undefined)
      ? lastPageOverride.prompt
      : page.prompt;
    const effectiveResponse = (isLast && lastPageOverride?.response !== undefined)
      ? lastPageOverride.response
      : page.response;

    if (effectivePrompt) {
      messages.push({ author: 'user', content: effectivePrompt });
    }
    if (effectiveResponse) {
      messages.push({ author: 'ai', content: effectiveResponse });
    }
  }

  return {
    context,
    messages,
  };
}
