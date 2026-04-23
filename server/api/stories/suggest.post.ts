import { ConversationalArchetype } from '@stegakir/aikit/archetypes/conversational';
import { Conversation, message } from '@stegakir/aikit/messages';
import { MemoryMessageStore } from '@stegakir/aikit/message-stores/memory';
import { unindent } from '@stegakir/aikit/utils';
import { mkdirSync, appendFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { resolveModel } from '#server/ai/models';

const archetype = new ConversationalArchetype({});

// --- Debug logging ---
const tmpDir = resolve(process.cwd(), 'tmp');
function log(file: string, msg: string) {
  try {
    mkdirSync(tmpDir, { recursive: true });
    appendFileSync(resolve(tmpDir, file), msg + '\n');
  } catch { /* best effort */ }
}

const SYSTEM_RANDOM = unindent(`
  You are a creative story idea generator.
  Generate exactly 5 diverse, creative story suggestions spanning different genres
  (e.g. fantasy, sci-fi, mystery, horror, romance, adventure, thriller).

  For each suggestion, output a <suggestion> block with these tags inside:
    <storyId> — a URL-safe slug, lowercase, hyphens (e.g. "the-lost-dungeon")</storyId>
    <title> — a catchy story title</title>
    <genre> — the genre</genre>
    <description> — a compelling premise, 2-3 sentences</description>

  Output ONLY the <suggestion> blocks with no other text.
`);

const SYSTEM_KEYWORDS = unindent(`
  You are a creative story idea generator.
  The user will provide keywords or a theme. Generate exactly 3 diverse story suggestions
  inspired by their input. Each suggestion must have a unique genre and tone.

  For each suggestion, output a <suggestion> block with these tags inside:
    <storyId> — a URL-safe slug, lowercase, hyphens (e.g. "the-lost-dungeon")</storyId>
    <title> — a catchy story title</title>
    <genre> — the genre</genre>
    <description> — a compelling premise, 2-3 sentences</description>

  Output ONLY the <suggestion> blocks with no other text.
`);

export default defineEventHandler(async (event) => {
  const body = await readBody(event);
  const prompt = body?.prompt as string | undefined;
  const hasPrompt = typeof prompt === 'string' && prompt.trim().length > 0;

  log('suggest.log', `[${new Date().toISOString()}] request prompt=${JSON.stringify(prompt)} hasPrompt=${hasPrompt}`);
  console.log('[suggest] request received, hasPrompt:', hasPrompt);

  const eventStream = createEventStream(event);

  const msgStore = new MemoryMessageStore();
  const conversation = new Conversation(msgStore, `suggest-${Date.now()}`);
  await conversation.push(
    message({ author: 'user', content: hasPrompt ? `Keywords/theme: ${prompt!.trim()}` : 'Generate random story ideas.' }),
  );

  const model = resolveModel('zai-org/glm-4.6v-flash');

  let chunkCount = 0;
  const t0 = Date.now();

  const streamLoop = (async () => {
    try {
      console.log('[suggest] calling archetype.prompt…');
      log('suggest.log', `[${new Date().toISOString()}] calling archetype.prompt…`);

      const stream = await archetype.prompt({
        model,
        conversation,
        persona: hasPrompt ? SYSTEM_KEYWORDS : SYSTEM_RANDOM,
      });

      console.log('[suggest] stream started');
      log('suggest.log', `[${new Date().toISOString()}] stream started`);

      let fullText = '';
      for await (const chunk of stream) {
        chunkCount++;
        const summary = `type=${chunk.type} ${JSON.stringify(chunk).slice(0, 300)}`;
        log('suggest-raw.log', `[${chunkCount}] ${summary}`);

        if (chunk.type === 'text-delta') {
          fullText += chunk.text;
          await eventStream.push({ event: 'text', data: JSON.stringify(chunk.text) });
        } else if (chunk.type === 'reasoning-delta') {
          if (chunk.text) {
            await eventStream.push({ event: 'reasoning', data: JSON.stringify(chunk.text) });
          }
        }
      }

      log('suggest-full.txt', fullText);
      log('suggest.log', `[${new Date().toISOString()}] stream ended chunks=${chunkCount} elapsed=${Date.now() - t0}ms`);
      console.log(`[suggest] stream ended, ${chunkCount} chunks, ${Date.now() - t0}ms`);

      await eventStream.push({ event: 'done', data: '' });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      log('suggest.log', `[${new Date().toISOString()}] ERROR: ${msg}`);
      console.error('[suggest] ERROR:', msg);
      await eventStream.push({ event: 'error', data: msg });
    } finally {
      await eventStream.close();
    }
  })();

  await Promise.all([eventStream.send(), streamLoop]);
});
