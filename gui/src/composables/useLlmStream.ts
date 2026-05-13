import { ConversationalArchetype } from '@stegakir/aikit/archetypes/conversational';
import type { Context } from '@stegakir/aikit/archetypes/conversational';
import { Conversation, MemoryMessageStore, message } from '@stegakir/aikit';
import type { ToolSet } from 'ai';
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
  type: 'text' | 'reasoning' | 'done' | 'error' | 'tool-call' | 'tool-result';
  data: string;
  finishReason?: string;
  usage?: LlmUsage;
}

export interface StreamLlmOptions {
  persona: string;
  messages: Array<{ author: string; content: string }>;
  model?: string;
  context?: Context;
  tools?: ToolSet;
}

export async function* streamLlm({
  persona,
  messages,
  model = DEFAULT_MODEL,
  context,
  tools,
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
    context,
    tools,
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
        case 'tool-call':
          yield {
            type: 'tool-call',
            data: JSON.stringify({ id: part.toolCallId, tool: part.toolName, args: 'input' in part ? part.input : undefined }),
          };
          break;
        case 'tool-result':
          yield {
            type: 'tool-result',
            data: JSON.stringify({ id: part.toolCallId, tool: part.toolName, result: 'output' in part ? part.output : undefined }),
          };
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
