/**
 * @deprecated This module has been decommissioned.
 *
 * All agent/LLM calls should be routed through the frontend via
 * POST /api/llm/prompt — keeping AI concerns isolated from endpoint
 * logic. This enables "bring-your-own-agent" models and premium agent servers.
 *
 * TODO: Reimplement session-based gameplay using the separated architecture.
 *       See AGENTS.md → "Agent / LLM Integration" for the pattern.
 */

export async function promptGameAgent(_sessionId: number, _userContent: string): Promise<string> {
  throw new Error(
    'promptGameAgent is deprecated. Route LLM calls through POST /api/llm/prompt instead. ' +
    'See AGENTS.md → "Agent / LLM Integration" for details.',
  );
}
