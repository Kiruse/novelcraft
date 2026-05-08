import type { DeepReadonly, Ref } from "vue";
import { buildMessages, LlmMessage } from "~/utils/llmHelpers";
import type { VignetteMeta, VignettePage } from "./useVignette";
import { PERSONA_PLATFORM } from "~/prompts";
import { LlmUsage, streamLlmFull, StreamLlmOptions } from "./useLlmStream";
import { useProfiles } from "./useProfiles";

export type GameStatus = 'idle' | 'streaming';

export interface PromptDebug {
  persona: string;
  messages: readonly { author: string; content: string }[];
  model?: string;
  context?: Record<string, unknown>;
  promptId?: string;
}

export interface UseGameOpts {
  meta: DeepReadonly<Ref<VignetteMeta, any>>;
  pages: DeepReadonly<Ref<VignettePage[], any>>;
}

export interface GameRunOpts {
  /** An optional, arbitrary prompt ID to pass to the `prompt` ref for identification. */
  promptId?: string;
  getMessages?(messages: LlmMessage[]): LlmMessage[];
  /** Optional text to prepend to the live `streamText`.
   * Useful for generating more text to an existing generation.
   * The returned `response` will include this prefix.
   */
  prependStreamText?: string;
}

export function useGame({ meta, pages }: UseGameOpts) {
  const status = ref<GameStatus>('idle');
  const prompt = ref<PromptDebug | undefined>(undefined);
  const streamText = ref('');
  const thoughts = ref('');
  const tokenUsage = ref<LlmUsage | undefined>();

  const { activeProfile } = useProfiles();

  async function run({
    promptId,
    getMessages = (msgs) => msgs,
    prependStreamText,
  }: GameRunOpts) {
    let response = prependStreamText ? prependStreamText + '\n\n' : '';
    status.value = 'streaming';
    streamText.value = response;
    thoughts.value = '';

    try {
      const { context, messages } = buildMessages({
        title: meta.value.title,
        description: meta.value.disposition.trim() || undefined,
        profile: activeProfile.value ?? undefined,
        pages: pages.value,
      });

      const streamOpts: StreamLlmOptions = {
        persona: PERSONA_PLATFORM,
        messages: getMessages(messages),
        context,
      };
      prompt.value = { promptId, ...streamOpts };

      const streamGen = streamLlmFull(streamOpts);
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

      return response;
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
