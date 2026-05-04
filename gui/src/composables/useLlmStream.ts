import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { DEFAULT_MODEL } from '~/prompts';

export interface StreamEvent {
  type: 'text' | 'reasoning' | 'done' | 'error';
  data: string;
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
  const unlisteners: UnlistenFn[] = [];
  const queue: StreamEvent[] = [];
  let done = false;

  unlisteners.push(
    await listen<string>('llm:text', (e) => {
      queue.push({ type: 'text', data: e.payload });
    }),
  );

  unlisteners.push(
    await listen<string>('llm:reasoning', (e) => {
      queue.push({ type: 'reasoning', data: e.payload });
    }),
  );

  unlisteners.push(
    await listen<string>('llm:error', (e) => {
      queue.push({ type: 'error', data: e.payload });
    }),
  );

  unlisteners.push(
    await listen<string>('llm:done', () => {
      done = true;
    }),
  );

  const cleanup = () => {
    unlisteners.forEach(un => un());
  };

  let invokeError: string | null = null;

  const invokePromise = invoke('prompt', {
    request: { model, messages, persona, context },
  }).catch((err) => {
    invokeError = err instanceof Error ? err.message : String(err);
    done = true;
  });

  try {
    while (!done || queue.length > 0) {
      const event = queue.shift();
      if (!event) {
        if (done) break;
        await new Promise((resolve) => setTimeout(resolve, 16));
        continue;
      }

      if (event.type === 'error') {
        yield event;
        cleanup();
        await invokePromise;
        return;
      }

      yield event;
    }

    if (invokeError) {
      yield { type: 'error', data: invokeError };
    } else {
      yield { type: 'done', data: '' };
    }
  } catch (err) {
    yield {
      type: 'error',
      data: err instanceof Error ? err.message : String(err),
    };
  } finally {
    cleanup();
  }
}
