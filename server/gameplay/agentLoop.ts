import { ConversationalArchetype } from '@stegakir/aikit/archetypes/conversational';
import { Conversation, message } from '@stegakir/aikit/messages';
import { MemoryMessageStore } from '@stegakir/aikit/message-stores/memory';
import { unindent } from '@stegakir/aikit/utils';
import { tool, zodSchema } from 'ai';
import type { ToolSet } from 'ai';
import { z } from 'zod';
import { db } from '#server/db';
import { gameSessionMessages, gameSessions, moduleRuntime } from '#server/db/schema/app';
import { eq, asc } from 'drizzle-orm';
import {
  getAllModules,
  getModule,
  type GameplaySession,
  type GameplayModuleContext,
} from './gameplayModule';
import { resolveModel } from '#server/ai/models';

const archetype = new ConversationalArchetype({});

const DM_PERSONA = unindent(`
  You are a Dungeon Master narrating an interactive story. You describe the world vividly,
  portray NPCs with distinct personalities, and react to player actions with consequences
  that feel natural and fair. You maintain continuity with established facts and keep the
  story engaging with meaningful choices. Keep responses concise but evocative.
`);

export async function promptGameAgent(sessionId: number, userContent: string): Promise<string> {
  // 1. Load & assemble the full gameplay session
  const session = await loadGameplaySession(sessionId);

  // 2. Reconstruct conversation from stored messages
  const msgStore = new MemoryMessageStore();
  const conversation = new Conversation(msgStore, `session-${sessionId}`);

  const dbMessages = await db
    .select()
    .from(gameSessionMessages)
    .where(eq(gameSessionMessages.gameSessionId, sessionId))
    .orderBy(asc(gameSessionMessages.createdAt));

  for (const msg of dbMessages) {
    await conversation.push(
      message({
        author: msg.role === 'agent' ? 'ai' : msg.role,
        content: msg.contents,
        timestamp: new Date(msg.createdAt),
      }),
    );
  }

  // 3. Save & push the new user message
  await db.insert(gameSessionMessages).values({
    gameSessionId: sessionId,
    role: 'user',
    contents: userContent,
  });

  await conversation.push(
    message({
      author: 'user',
      content: userContent,
    }),
  );

  // 4. Collect knowledge from all modules
  const knowledge: Record<string, string> = {};
  for (const modState of session.modules) {
    const modDef = getModule(modState.type);
    if (!modDef) continue;
    const ctx = {
      session,
      module: modDef,
      config: modState.config,
      state: modState.state,
    } as GameplayModuleContext<any, any, any>;
    const moduleKnowledge = modDef.getKnowledge(ctx);
    for (const [key, value] of Object.entries(moduleKnowledge)) {
      knowledge[key] = typeof value === 'string' ? value : JSON.stringify(value);
    }
  }

  // 5. Build AI SDK tools from module tool definitions, with state tracking
  const stateTracker = new Map<string, Record<string, unknown>>();
  const tools = buildToolSet(session, stateTracker);

  // 6. Run the agent
  const model = resolveModel('zai-org/glm-4.6v-flash');
  const stream = await archetype.prompt({
    model,
    conversation,
    persona: DM_PERSONA,
    context: knowledge,
    tools: Object.keys(tools).length > 0 ? tools : undefined,
  });

  // Consume the stream — archetype returns an AsyncGenerator that yields text
  // parts and resolves to the final StreamTextResult.
  let text = '';
  for await (const chunk of stream) {
    if (chunk.type === 'text-delta') {
      text += chunk.text;
    }
  }

  // 7. Save agent response
  await db.insert(gameSessionMessages).values({
    gameSessionId: sessionId,
    role: 'agent',
    contents: text || '(no response)',
  });

  // 8. Persist mutated module states
  if (stateTracker.size > 0) {
    for (const [moduleType, newState] of stateTracker) {
      await db
        .update(moduleRuntime)
        .set({ data: newState })
        .where(eq(moduleRuntime.moduleId, moduleType));
    }
  }

  return text || '(no response)';
}

// --- Session loading ---

export interface SessionModule {
  type: string;
  config: unknown;
  state: unknown;
}

async function loadGameplaySession(sessionId: number): Promise<GameplaySession & { modules: SessionModule[] }> {
  const session = await db.query.gameSessions.findFirst({
    where: eq(gameSessions.id, sessionId),
    with: {
      moduleRuntime: true,
      story: true,
    },
  });

  if (!session) throw new Error(`Game session ${sessionId} not found`);

  // Story modules is a JSON blob typed as unknown[] but is actually Record<moduleType, config>
  const storyModules = (session.story.modules ?? {}) as unknown as Record<string, unknown>;
  const runtimeStates = new Map(
    session.moduleRuntime.map((r) => [r.moduleId, r.data]),
  );

  const modules: SessionModule[] = Object.entries(storyModules).map(
    ([type, config]) => ({
      type,
      config,
      state: runtimeStates.get(type) ?? null,
    }),
  );

  return {
    record: session,
    modules,
  };
}

// --- Tool building ---

function buildToolSet(
  session: GameplaySession & { modules: SessionModule[] },
  stateTracker: Map<string, Record<string, unknown>>,
): ToolSet {
  const tools: ToolSet = {};

  for (const modState of session.modules) {
    const modDef = getModule(modState.type);
    if (!modDef?.tools) continue;

    for (const toolDef of modDef.tools) {
      const capturedType = modState.type;
      tools[toolDef.name] = tool({
        description: toolDef.description,
        inputSchema: zodSchema(toolDef.parameters ?? z.object({})),
        execute: async (params: any) => {
          const ctx = {
            session,
            module: modDef,
            config: modState.config,
            state: modState.state,
          } as GameplayModuleContext<any, any, any>;
          const result = await toolDef.execute(params, ctx);
          if (result.success) {
            stateTracker.set(capturedType, result.state);
            // Update in-memory state so subsequent tool calls in the same
            // multi-step run see the updated state
            modState.state = result.state;
          }
          return result.success ? result.state : { error: result.error };
        },
      });
    }
  }

  return tools;
}
