import type { GamePage } from '~/utils/msgUtils';
import type { Profile } from '~/composables/useProfiles';

export interface Context {
  [key: string]: string | Context | Record<string, string | unknown> | undefined;
}

export interface LlmMessage {
  author: string;
  content: string;
}

export interface BuildMessagesOpts {
  title: string;
  description: string | null | undefined;
  profile?: Profile;
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
  profile,
  pages,
  pageIndex = pages.length - 1,
  lastPageOverride,
}: BuildMessagesOpts): BuildMessagesResult {
  const context: Context = {
    story: {
      title,
    },
    ...getProfileContext(profile),
  };

  if (description) (context.story as Record<string, string>).description = description;

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

function getProfileContext(profile?: Profile) {
  if (!profile) return {};
  const fields = Object.entries(profile.fields).filter(([, v]) => v.trim());
  if (fields.length === 0) return {};
  return {
    user: Object.fromEntries(fields) as Record<string, unknown>,
  };
}
