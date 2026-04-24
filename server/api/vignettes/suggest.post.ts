import { ConversationalArchetype } from '@stegakir/aikit/archetypes/conversational';
import { Conversation, message } from '@stegakir/aikit/messages';
import { MemoryMessageStore } from '@stegakir/aikit/message-stores/memory';
import { unindent } from '@stegakir/aikit/utils';
import { resolveModel } from '#server/ai/models';

const archetype = new ConversationalArchetype({});

const SYSTEM_PROMPT = unindent(`
  You are a creative story premise generator for quick interactive vignettes.
  The user will provide a disposition — either a few keywords, a theme, or a full paragraph
  describing the kind of story they want to experience.

  Generate exactly 3 diverse story suggestions. Each must have a unique genre and tone.
  For each suggestion, output a <suggestion> block with these tags inside:
    <title> — a short, catchy title (3-6 words)</title>
    <genre> — the genre</genre>
    <description> — a vivid 2-3 sentence premise that sets the scene and ends with a hook.
    Write descriptions in second person ("You..."). Make them evocative and specific.</description>

  Output ONLY the <suggestion> blocks with no other text.
`);

export default defineEventHandler(async (event) => {
  const body = await readBody(event);
  const prompt = (body?.prompt as string | undefined) ?? '';

  const eventStream = createEventStream(event);

  const msgStore = new MemoryMessageStore();
  const conversation = new Conversation(msgStore, `vignette-suggest-${Date.now()}`);
  await conversation.push(
    message({
      author: 'user',
      content: prompt.trim().length > 0
        ? `Here is my disposition:\n\n${prompt.trim()}\n\nGenerate 3 story suggestions based on this.`
        : 'Generate 3 random creative story suggestions for interactive vignettes. Surprise me with variety.',
    }),
  );

  const model = resolveModel('zai-org/glm-4.6v-flash');

  const streamLoop = (async () => {
    try {
      const stream = await archetype.prompt({
        model,
        conversation,
        persona: SYSTEM_PROMPT,
      });

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
      console.error('[vignette-suggest] ERROR:', msg);
      await eventStream.push({ event: 'error', data: msg });
    } finally {
      await eventStream.close();
    }
  })();

  await Promise.all([eventStream.send(), streamLoop]);
});
