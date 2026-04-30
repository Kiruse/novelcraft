import { ConversationalArchetype } from '@stegakir/aikit/archetypes/conversational';
import { Conversation, message } from '@stegakir/aikit/messages';
import { MemoryMessageStore } from '@stegakir/aikit/message-stores/memory';
import { auth } from '#server/auth/config';
import { resolveModel } from '#server/ai/models';
import { z } from 'zod';

const archetype = new ConversationalArchetype({});

const bodySchema = z.object({
  model: z.string().min(1),
  messages: z.array(z.object({
    author: z.string(),
    content: z.string().min(1),
  })).min(1),
  persona: z.string().min(1),
  context: z.record(z.string(), z.string()).optional(),
});

export default defineEventHandler(async (event) => {
  // --- Auth gate ---
  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  // --- Validate input ---
  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({
      statusCode: 400,
      statusMessage: 'Invalid request body',
      data: z.treeifyError(parsed.error),
    });
  }

  const { model: modelId, messages: history, persona, context } = parsed.data;
  const model = resolveModel(modelId);

  // --- Build conversation ---
  const msgStore = new MemoryMessageStore();
  const conversation = new Conversation(msgStore, `prompt-${Date.now()}`);
  for (const entry of history) {
    await conversation.push(message({ author: entry.author, content: entry.content }));
  }

  // --- Stream response ---
  const eventStream = createEventStream(event);

  const streamLoop = (async () => {
    try {
      const stream = await archetype.prompt({ model, conversation, persona, context });

      for await (const chunk of stream) {
        if (chunk.type === 'text-delta') {
          await eventStream.push({ event: 'text', data: JSON.stringify(chunk.text) });
        } else if (chunk.type === 'reasoning-delta') {
          if (chunk.text) {
            await eventStream.push({ event: 'reasoning', data: JSON.stringify(chunk.text) });
          }
        }
      }

      await eventStream.push({ event: 'done', data: '' });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[llm/prompt] ERROR:', msg);
      await eventStream.push({ event: 'error', data: msg });
    } finally {
      await eventStream.close();
    }
  })();

  await Promise.all([eventStream.send(), streamLoop]);
});
