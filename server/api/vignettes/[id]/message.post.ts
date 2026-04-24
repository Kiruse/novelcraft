import { ConversationalArchetype } from '@stegakir/aikit/archetypes/conversational';
import { Conversation, message } from '@stegakir/aikit/messages';
import { MemoryMessageStore } from '@stegakir/aikit/message-stores/memory';
import { unindent } from '@stegakir/aikit/utils';
import { db } from '#server/db';
import { stories, gameSessions, gameSessionMessages } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq, and, desc, asc } from 'drizzle-orm';
import { z } from 'zod';
import { resolveModel } from '#server/ai/models';

const archetype = new ConversationalArchetype({});

const PLAY_PERSONA = unindent(`
  You are a vivid, immersive interactive fiction narrator continuing a vignette.
  You respond to the player's actions with consequences, new developments, and sensory detail.
  Rules:
  - Write in second person ("You...")
  - Keep responses focused — 1-3 paragraphs per response
  - React meaningfully to what the player does
  - Maintain continuity with everything established so far
  - Always leave room for the player to make a choice
  - Don't make choices for the player
  - Don't resolve the story too quickly — let it breathe
`);

const bodySchema = z.object({
  content: z.string().min(1).max(4000),
  sessionId: z.number().int().positive(),
});

export default defineEventHandler(async (event) => {
  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const id = getRouterParam(event, 'id');
  if (!id) {
    throw createError({ statusCode: 400, statusMessage: 'Vignette ID is required' });
  }

  // Verify the vignette story
  const story = await db.query.stories.findFirst({
    where: and(
      eq(stories.id, parseInt(id)),
      eq(stories.isVignette, true),
    ),
  });

  if (!story) {
    throw createError({ statusCode: 404, statusMessage: 'Vignette not found' });
  }
  if (story.authorId !== authSession.user.id) {
    throw createError({ statusCode: 403, statusMessage: 'Not your vignette' });
  }

  const body = await readBody(event);
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid input' });
  }

  // Verify the game session belongs to this vignette + user
  const gameSession = await db.query.gameSessions.findFirst({
    where: and(
      eq(gameSessions.id, parsed.data.sessionId),
      eq(gameSessions.storyId, story.id),
      eq(gameSessions.playerId, authSession.user.id),
    ),
  });

  if (!gameSession) {
    throw createError({ statusCode: 404, statusMessage: 'Session not found' });
  }

  // Save user message
  await db.insert(gameSessionMessages).values({
    gameSessionId: gameSession.id,
    role: 'user',
    contents: parsed.data.content,
  });

  const eventStream = createEventStream(event);

  const streamLoop = (async () => {
    try {
      // Reconstruct conversation from stored messages
      const msgStore = new MemoryMessageStore();
      const conversation = new Conversation(msgStore, `vignette-${story.id}`);

      // Push context
      const context = story.description
        ? `Title: ${story.title}\nPremise: ${story.description}`
        : `Title: ${story.title}`;
      await conversation.push(
        message({ author: 'user', content: `[Context] ${context}` }),
      );
      await conversation.push(
        message({ author: 'ai', content: '(Vignette started)' }),
      );

      // Load all previous messages
      const dbMsgs = await db
        .select()
        .from(gameSessionMessages)
        .where(eq(gameSessionMessages.gameSessionId, gameSession.id))
        .orderBy(asc(gameSessionMessages.createdAt));

      for (const msg of dbMsgs) {
        await conversation.push(
          message({
            author: msg.role === 'agent' ? 'ai' : msg.role,
            content: msg.contents,
          }),
        );
      }

      const model = resolveModel('zai-org/glm-4.6v-flash');

      const stream = await archetype.prompt({
        model,
        conversation,
        persona: PLAY_PERSONA,
      });

      let fullText = '';
      for await (const chunk of stream) {
        if (chunk.type === 'text-delta') {
          fullText += chunk.text;
          await eventStream.push({ event: 'text', data: JSON.stringify(chunk.text) });
        } else if (chunk.type === 'reasoning-delta') {
          if (chunk.text) {
            await eventStream.push({ event: 'reasoning', data: JSON.stringify(chunk.text) });
          }
        }
      }

      // Save agent response
      if (fullText.trim()) {
        await db.insert(gameSessionMessages).values({
          gameSessionId: gameSession.id,
          role: 'agent',
          contents: fullText,
        });
      }

      await eventStream.push({ event: 'done', data: '' });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[vignette-message] ERROR:', msg);
      await eventStream.push({ event: 'error', data: msg });
    } finally {
      await eventStream.close();
    }
  })();

  await Promise.all([eventStream.send(), streamLoop]);
});
