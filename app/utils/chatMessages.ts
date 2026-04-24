import type { ChatMessage } from '~/components/ChatArea.vue';

/** Raw message shape stored in the database. */
export interface RawMessage {
  id: number | string;
  role: 'user' | 'agent' | 'system';
  contents: string;
}

/**
 * Convert raw agent/user/system messages into ChatMessage[] for ChatArea.
 *
 * Current mapping:
 *  - user  → author = "You" (player speech bubble)
 *  - agent → author = null  (narrator prose)
 *  - system → filtered out
 *
 * This will grow more complex as the story system evolves
 * (character attribution, multi-speaker blocks, etc.).
 */
export function toChatMessages(messages: RawMessage[]): ChatMessage[] {
  return messages
    .filter(m => m.role !== 'system')
    .map(m => ({
      id: m.id,
      contents: m.contents,
      author: m.role === 'user' ? 'You' : null,
    }));
}
