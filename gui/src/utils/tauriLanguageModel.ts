import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  LanguageModelV3,
  LanguageModelV3CallOptions,
  LanguageModelV3StreamResult,
  LanguageModelV3GenerateResult,
  LanguageModelV3StreamPart,
  LanguageModelV3Prompt,
  LanguageModelV3FunctionTool,
  LanguageModelV3ProviderTool,
  LanguageModelV3Content,
  LanguageModelV3FinishReason,
  LanguageModelV3Usage,
} from '@ai-sdk/provider';
import { commands } from '~/bindings';

interface TauriMessage {
  author: string;
  content: string;
  tool_call_id?: string;
  tool_calls?: Array<{
    id: string;
    type: string;
    function: { name: string; arguments: string };
  }>;
}

interface TauriTool {
  name: string;
  description?: string;
  parameters?: unknown;
}

interface TauriToolCallDelta {
  index: number;
  id?: string;
  name?: string;
  arguments_delta: string;
}

interface TauriDonePayload {
  finish_reason: string;
  usage?: { prompt_tokens: number; completion_tokens: number; total_tokens: number };
}

function convertToolResultOutput(output: { type: string; value?: unknown; reason?: string }): string {
  switch (output.type) {
    case 'text':
    case 'error-text':
      return String(output.value ?? '');
    case 'json':
    case 'error-json':
      return JSON.stringify(output.value);
    case 'execution-denied':
      return `Execution denied: ${output.reason ?? 'no reason'}`;
    case 'content': {
      const items = output.value as Array<{ type: string; text?: string }> | undefined;
      return (items ?? [])
        .filter((p) => p.type === 'text')
        .map((p) => p.text ?? '')
        .join('');
    }
    default:
      return JSON.stringify(output);
  }
}

function convertMessages(prompt: LanguageModelV3Prompt): TauriMessage[] {
  return prompt.map((msg) => {
    switch (msg.role) {
      case 'system':
        return { author: 'system', content: msg.content };

      case 'user': {
        const text = msg.content
          .filter((p) => p.type === 'text')
          .map((p) => p.text)
          .join('');
        return { author: 'user', content: text };
      }

      case 'assistant': {
        const textParts: string[] = [];
        const toolCalls: NonNullable<TauriMessage['tool_calls']> = [];

        for (const part of msg.content) {
          switch (part.type) {
            case 'text':
              textParts.push(part.text);
              break;
            case 'tool-call':
              toolCalls.push({
                id: part.toolCallId,
                type: 'function',
                function: {
                  name: part.toolName,
                  arguments: typeof part.input === 'string' ? part.input : JSON.stringify(part.input),
                },
              });
              break;
          }
        }

        const result: TauriMessage = { author: 'ai', content: textParts.join('') };
        if (toolCalls.length > 0) result.tool_calls = toolCalls;
        return result;
      }

      case 'tool': {
        const results: string[] = [];
        let toolCallId = '';

        for (const part of msg.content) {
          if (part.type === 'tool-result') {
            toolCallId = part.toolCallId;
            results.push(convertToolResultOutput(part.output));
          }
        }

        return {
          author: 'tool',
          content: results.join('\n'),
          tool_call_id: toolCallId,
        };
      }
    }
  });
}

function convertTools(tools: Array<LanguageModelV3FunctionTool | LanguageModelV3ProviderTool>): TauriTool[] {
  return tools
    .filter((t): t is LanguageModelV3FunctionTool => t.type === 'function')
    .map((t) => {
      const result: TauriTool = { name: t.name };
      if (t.description) result.description = t.description;
      if (t.inputSchema) result.parameters = t.inputSchema;
      return result;
    });
}

function mapFinishReason(raw: string): LanguageModelV3FinishReason['unified'] {
  let normalized = raw.replaceAll('_', '-');
  if (['stop', 'length', 'content-filter', 'tool-calls', 'error'].includes(normalized))
    return normalized as any;
  return 'other';
}

const emptyUsage = (): LanguageModelV3Usage => ({
  inputTokens: { total: undefined, noCache: undefined, cacheRead: undefined, cacheWrite: undefined },
  outputTokens: { total: undefined, text: undefined, reasoning: undefined },
});

export class TauriLanguageModel implements LanguageModelV3 {
  readonly specificationVersion = 'v3' as const;
  readonly provider = 'tauri' as const;
  readonly modelId: string;
  readonly supportedUrls: Record<string, RegExp[]> = {};

  constructor(modelId: string) {
    this.modelId = modelId;
  }

  async doStream(options: LanguageModelV3CallOptions): Promise<LanguageModelV3StreamResult> {
    const requestId = crypto.randomUUID();
    const unlisteners: UnlistenFn[] = [];
    const cleanups: (() => void)[] = [];
    let closed = false;

    const cleanup = () => {
      if (closed) return;
      closed = true;
      for (const unlisten of unlisteners) unlisten();
      for (const fn of cleanups) fn();
    };

    const messages = convertMessages(options.prompt);
    const tools = options.tools ? convertTools(options.tools) : undefined;

    let textId = `text-${Date.now()}`;
    let textStarted = false;
    let reasoningId = `reasoning-${Date.now()}`;
    let reasoningStarted = false;
    const toolCallMap = new Map<number, { id: string; name: string; started: boolean; ended: boolean }>();

    let controller: ReadableStreamDefaultController<LanguageModelV3StreamPart> | null = null;

    const enqueue = (part: LanguageModelV3StreamPart) => {
      if (closed || !controller) return;
      try {
        controller.enqueue(part);
      } catch {
        // Stream already closed
      }
    };

    const stream = new ReadableStream<LanguageModelV3StreamPart>({
      start(c) {
        controller = c;
      },
      cancel() {
        cleanup();
      },
    });

    unlisteners.push(
      await listen<string>(`llm:text:${requestId}`, (e) => {
        if (!textStarted) {
          enqueue({ type: 'text-start', id: textId });
          textStarted = true;
        }
        enqueue({ type: 'text-delta', id: textId, delta: e.payload });
      }),
    );

    unlisteners.push(
      await listen<string>(`llm:reasoning:${requestId}`, (e) => {
        if (!reasoningStarted) {
          enqueue({ type: 'reasoning-start', id: reasoningId });
          reasoningStarted = true;
        }
        enqueue({ type: 'reasoning-delta', id: reasoningId, delta: e.payload });
      }),
    );

    unlisteners.push(
      await listen<TauriToolCallDelta>(`llm:tool_call:${requestId}`, (e) => {
        const { index, id, name, arguments_delta } = e.payload;
        let tc = toolCallMap.get(index);
        if (!tc) {
          tc = { id: id ?? `tool-${index}`, name: name ?? '', started: false, ended: false };
          toolCallMap.set(index, tc);
        }
        if (id) tc.id = id;
        if (name) tc.name = name;

        if (!tc.started) {
          enqueue({ type: 'tool-input-start', id: tc.id, toolName: tc.name });
          tc.started = true;
        }
        enqueue({ type: 'tool-input-delta', id: tc.id, delta: arguments_delta });
      }),
    );

    unlisteners.push(
      await listen<string>(`llm:error:${requestId}`, (e) => {
        enqueue({ type: 'error', error: e.payload });
        enqueue({
          type: 'finish',
          finishReason: { unified: 'error', raw: 'error' },
          usage: emptyUsage(),
        });
        try { controller?.close(); } catch {}
        cleanup();
      }),
    );

    unlisteners.push(
      await listen<TauriDonePayload>(`llm:done:${requestId}`, (e) => {
        const { finish_reason, usage } = e.payload;

        for (const [, tc] of toolCallMap) {
          if (!tc.ended) {
            enqueue({ type: 'tool-input-end', id: tc.id });
            tc.ended = true;
          }
        }

        enqueue({
          type: 'finish',
          finishReason: { unified: mapFinishReason(finish_reason), raw: finish_reason },
          usage: {
            inputTokens: {
              total: usage?.prompt_tokens,
              noCache: undefined,
              cacheRead: undefined,
              cacheWrite: undefined,
            },
            outputTokens: {
              total: usage?.completion_tokens,
              text: undefined,
              reasoning: undefined,
            },
          },
        });
        try { controller?.close(); } catch {}
        cleanup();
      }),
    );

    if (options.abortSignal) {
      const onAbort = () => cleanup();
      options.abortSignal.addEventListener('abort', onAbort, { once: true });
      cleanups.push(() => options.abortSignal!.removeEventListener('abort', onAbort));
    }

    commands.prompt({
      model: this.modelId,
      messages,
      tools,
      request_id: requestId,
    }).catch(() => {
      cleanup();
    });

    return { stream };
  }

  async doGenerate(options: LanguageModelV3CallOptions): Promise<LanguageModelV3GenerateResult> {
    const { stream } = await this.doStream(options);
    const reader = stream.getReader();

    let response = '';
    let reasoning = '';
    const toolCalls = new Map<string, { name: string; args: string }>();
    let finishReason: LanguageModelV3FinishReason = { unified: 'stop', raw: undefined };
    let usage: LanguageModelV3Usage = emptyUsage();

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      switch (value.type) {
        case 'text-delta':
          response += value.delta;
          break;
        case 'reasoning-delta':
          reasoning += value.delta;
          break;
        case 'tool-input-start':
          toolCalls.set(value.id, { name: value.toolName, args: '' });
          break;
        case 'tool-input-delta': {
          const tc = toolCalls.get(value.id);
          if (tc) tc.args += value.delta;
          break;
        }
        case 'finish':
          finishReason = value.finishReason;
          usage = value.usage;
          break;
      }
    }

    const content: LanguageModelV3Content[] = [];

    if (reasoning.trim())
      content.push({ type: 'reasoning', text: reasoning.trim() });
    if (response.trim())
      content.push({ type: 'text', text: response.trim() });

    for (const [id, { name, args }] of toolCalls) {
      content.push({ type: 'tool-call', toolCallId: id, toolName: name, input: args });
    }

    return { content, finishReason, usage, warnings: [] };
  }
}

export const createTauriModel = (modelId: string): LanguageModelV3 =>
  new TauriLanguageModel(modelId);
