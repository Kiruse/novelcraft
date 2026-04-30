import { DEFAULT_MODEL } from '#shared/prompts';

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

function parseSseChunk(buffer: string): { events: StreamEvent[]; remainder: string } {
  const events: StreamEvent[] = [];
  const parts = buffer.split('\n\n');
  const remainder = parts.pop() ?? '';

  for (const part of parts) {
    const lines = part.split('\n');
    const eventType = lines.find(l => l.startsWith('event: '))?.slice(7).trim();
    const dataLine = lines.find(l => l.startsWith('data: '))?.slice(6) ?? '';
    let data = '';
    if (dataLine) {
      try { data = JSON.parse(dataLine) as string; } catch { data = dataLine; }
    }

    if (eventType && eventType !== 'done') {
      events.push({ type: eventType as StreamEvent['type'], data });
    }
  }

  return { events, remainder };
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
  const response = await fetch('/api/llm/prompt', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ model, messages, persona, context }),
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
    const { events, remainder } = parseSseChunk(buffer);
    buffer = remainder;

    for (const event of events) {
      yield event;
    }
  }
}
