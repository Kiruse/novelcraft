import { ConversationalArchetype } from '@stegakir/aikit/archetypes/conversational';
import { Conversation, message } from '@stegakir/aikit/messages';
import { MemoryMessageStore } from '@stegakir/aikit/message-stores/memory';
import { unindent } from '@stegakir/aikit/utils';
import { db } from '#server/db';
import { stories, gameSessions, gameSessionMessages, moduleRuntime } from '#server/db/schema/app';
import { auth } from '#server/auth/config';
import { eq, and } from 'drizzle-orm';
import { resolveModel } from '#server/ai/models';

const archetype = new ConversationalArchetype({});

const OPENING_PROMPT = unindent(`
  You are a vivid, immersive interactive fiction narrator.
  The player has provided a premise for a vignette — a short, self-contained interactive story.
  Your job is to deliver an opening scene that:
  - Immediately places the player in the scene using second person ("You...")
  - Sets the atmosphere with sensory details
  - Introduces the central tension or mystery
  - Ends with a clear moment of choice or action for the player
  - Is 2-4 paragraphs long
  - Does NOT make any choices for the player — stop at the decision point

  Keep the tone evocative but grounded. Avoid purple prose.
`);

export default defineEventHandler(async (event) => {
  const authSession = await auth.api.getSession({ headers: event.headers });
  if (!authSession?.user) {
    throw createError({ statusCode: 401, statusMessage: 'Not authenticated' });
  }

  const id = getRouterParam(event, 'id');
  if (!id) {
    throw createError({ statusCode: 400, statusMessage: 'Vignette ID is required' });
  }

  // Load and verify the vignette story
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

  // Check if there's already a session for this vignette
  const existingSession = await db.query.gameSessions.findFirst({
    where: and(
      eq(gameSessions.storyId, story.id),
      eq(gameSessions.playerId, authSession.user.id),
    ),
  });

  if (existingSession) {
    throw createError({ statusCode: 400, statusMessage: 'Vignette already started' });
  }

  // Create the game session
  const sessionRows = await db
    .insert(gameSessions)
    .values({
      playerId: authSession.user.id,
      storyId: story.id,
      data: {},
    })
    .returning();
  const gameSession = sessionRows[0]!;

  // Initialize system_prompt module with the disposition as prompt
  const disposition = story.description ?? '';
  await db.insert(moduleRuntime).values({
    gameSessionId: gameSession.id,
    moduleId: 'system_prompt',
    data: { version: 1 },
  });

  // Build context for the opening
  const context = story.description
    ? `Title: ${story.title}\n\nPremise:\n${story.description}\n\nAdditional context:\n${disposition}`
    : `Setup:\n${disposition}`;

  const eventStream = createEventStream(event);

  const streamLoop = (async () => {
    try {
      const msgStore = new MemoryMessageStore();
      const conversation = new Conversation(msgStore, `vignette-${story.id}`);

      await conversation.push(
        message({
          author: 'user',
          content: `Start the vignette based on this setup:\n\n${context}`,
        }),
      );

      const model = resolveModel('zai-org/glm-4.6v-flash');

      const stream = await archetype.prompt({
        model,
        conversation,
        persona: OPENING_PROMPT,
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

      // Save the opening narration as an agent message
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
      console.error('[vignette-start] ERROR:', msg);
      await eventStream.push({ event: 'error', data: msg });
    } finally {
      await eventStream.close();
    }
  })();

  await Promise.all([eventStream.send(), streamLoop]);
});
