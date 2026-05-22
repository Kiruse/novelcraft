import { Context } from "@stegakir/aikit/archetypes/conversational";
import type { DeepReadonly } from "vue";
import {
  type GameplaySession,
  type ToolCallRecord,
  createDefaultRegistry,
} from "~/gameplay";
import { PERSONA_PLATFORM } from "~/prompts";
import type { GameState, MaybeDeepReadonly, ReadableRef } from "~/utils";
import { type LlmUsage, streamLlm, type StreamLlmOptions } from "./useLlmStream";
import { type Profile, useProfiles } from "./useProfiles";
import type { VignetteMeta, VignettePage } from "./useVignette";

export type GameStatus = 'idle' | 'streaming';

export interface PromptDebug {
  persona: string;
  messages: readonly { author: string; content: string }[];
  model?: string;
  context?: Record<string, unknown>;
  promptId?: string;
}

export interface UseGameOpts {
  meta: ReadableRef<VignetteMeta>;
  pages: ReadableRef<VignettePage[]>;
}

export interface GameRunOpts {
  session: DeepReadonly<GameplaySession>;
  /** An optional, arbitrary prompt ID to pass to the `prompt` ref for identification. */
  promptId?: string;
  getMessages?(messages: LlmMessage[]): LlmMessage[];
  /** Optional text to prepend to the live `streamText`.
   * Useful for generating more text to an existing generation.
   * The returned `response` will include this prefix.
   */
  prependStreamText?: string;
}

export interface LlmMessage {
  author: string;
  content: string;
}

const registry = createDefaultRegistry();

export function useGame({ meta, pages }: UseGameOpts) {
  const status = ref<GameStatus>('idle');
  const prompt = ref<PromptDebug | undefined>(undefined);
  const streamText = ref('');
  const thoughts = ref('');
  const tokenUsage = ref<LlmUsage | undefined>();

  const { activeProfile } = useProfiles();

  async function run({
    session,
    promptId,
    getMessages = (msgs) => msgs,
    prependStreamText,
  }: GameRunOpts) {
    let response = prependStreamText ? prependStreamText + '\n\n' : '';
    status.value = 'streaming';
    streamText.value = response;
    thoughts.value = '';

    try {
      const toolCalls: ToolCallRecord[] = [];
      const state: GameState = { ...session.state };

      const tools = registry.getToolSet(session, (tool, params, moduleType, newState) => {
        toolCalls.push({ tool, params });
        state[moduleType] = newState;
      });

      const { context, messages } = buildMessages({
        session,
        title: meta.value.title,
        description: meta.value.description?.trim() || undefined,
        profile: activeProfile.value ?? undefined,
        pages: pages.value,
      });

      const streamOpts: StreamLlmOptions = {
        persona: PERSONA_PLATFORM,
        messages: getMessages(messages),
        context,
        tools,
      };
      prompt.value = { promptId, ...streamOpts };

      const streamGen = streamLlm(streamOpts);
      for await (const chunk of streamGen) {
        if (chunk.usage) tokenUsage.value = chunk.usage
        switch (chunk.type) {
          case 'reasoning':
            thoughts.value += chunk.data;
            break;
          case 'text':
            streamText.value = response += chunk.data;
            break;
        }
      }

      return { response, toolCalls, state };
    } finally {
      status.value = 'idle';
      streamText.value = '';
    }
  }

  return {
    status: readonly(status),
    streamText: readonly(streamText),
    thoughts: readonly(thoughts),
    tokenUsage: readonly(tokenUsage),
    prompt: readonly(prompt),
    run,
  };
}

interface BuildMessagesOpts {
  session?: GameplaySession;
  title: string;
  description?: string | null;
  profile?: Profile;
  pages: MaybeDeepReadonly<VignettePage[]>;
  pageIndex?: number;
  lastPageOverride?: { prompt?: string | null; response?: string | null };
}

function buildMessages({
  session,
  title,
  description = undefined,
  profile,
  pages,
  pageIndex = pages.length - 1,
  lastPageOverride,
}: BuildMessagesOpts) {
  const context: Context = {
    story: {
      title,
    },
    ...getProfileContext(profile),
  };

  if (description) (context.story as Record<string, string>).description = description;

  if (session) {
    const modulesKnowledge: Record<string, Context> = {};
    for (const mod of Object.values(registry.getAll())) {
      const knowledge = session.state[mod.type] ? mod.getKnowledge(readonly({
        session,
        module: mod,
        state: session.state[mod.type],
      })) : undefined;
      if (knowledge && Object.keys(knowledge).length > 0) {
        modulesKnowledge[mod.type] = knowledge as Context;
      }
    }
    if (Object.keys(modulesKnowledge).length > 0) {
      context.modules = modulesKnowledge;
    }
  }

  pages = pages.slice(0, pageIndex + 1);

  const messages: LlmMessage[] = [];

  for (let i = 0; i < pages.length; i++) {
    const isLast = i === pages.length - 1;
    const page = pages[i]!;

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

function getProfileContext(profile?: Profile): Context {
  if (!profile) return {};
  const fields = Object.entries(profile.fields).filter(([, v]) => v.trim());
  if (fields.length === 0) return {};
  return {
    user: Object.fromEntries(fields),
  };
}
