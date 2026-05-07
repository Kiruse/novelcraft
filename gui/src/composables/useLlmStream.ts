import { ConversationalArchetype } from '@stegakir/aikit/archetypes/conversational';
import type { Context } from '@stegakir/aikit/archetypes/conversational';
import { Conversation, MemoryMessageStore, message } from '@stegakir/aikit';
import { createTauriModel } from '~/utils/tauriLanguageModel';
import { DEFAULT_MODEL } from '~/prompts';

export interface LlmUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

export interface LlmDonePayload {
  finish_reason: string;
  usage?: LlmUsage;
}

export interface StreamEvent {
  type: 'text' | 'reasoning' | 'done' | 'error';
  data: string;
  finishReason?: string;
  usage?: LlmUsage;
}

export interface StreamLlmOptions {
  persona: string;
  messages: Array<{ author: string; content: string }>;
  model?: string;
  context?: Record<string, unknown>;
}

export async function* streamLlm(options: StreamLlmOptions): AsyncGenerator<string> {
  for await (const event of streamLlmFull(options)) {
    if (event.type === 'text') {
      yield event.data;
    }
  }
}

export async function* streamLlmFull({
  persona,
  messages,
  model = DEFAULT_MODEL,
  context,
}: StreamLlmOptions): AsyncGenerator<StreamEvent> {
  const store = new MemoryMessageStore();
  const conversation = new Conversation(store, crypto.randomUUID());

  for (const msg of messages) {
    await conversation.push(message({
      author: msg.author,
      content: msg.content,
    }));
  }

  const archetype = new ConversationalArchetype({
    persona,
    context: context as Context | undefined,
  });

  try {
    const result = await archetype.prompt({
      model: createTauriModel(model),
      conversation,
    });

    for await (const part of result) {
      switch (part.type) {
        case 'text-delta':
          yield { type: 'text', data: part.text };
          break;
        case 'reasoning-delta':
          yield { type: 'reasoning', data: part.text };
          break;
        case 'finish':
          yield {
            type: 'done',
            data: '',
            finishReason: part.rawFinishReason,
            usage: {
              prompt_tokens: part.totalUsage.inputTokens ?? 0,
              completion_tokens: part.totalUsage.outputTokens ?? 0,
              total_tokens: part.totalUsage.totalTokens ?? 0,
            },
          };
          break;
        case 'error':
          yield { type: 'error', data: String(part.error) };
          break;
      }
    }
  } catch (err) {
    yield {
      type: 'error',
      data: err instanceof Error ? err.message : String(err),
    };
  }
}
