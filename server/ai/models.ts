import { createOpenAICompatible } from '@ai-sdk/openai-compatible';
import type { LanguageModelV3 } from '@ai-sdk/provider';

/**
 * Hard-coded model configuration.
 * Each key is a model name used throughout the app.
 * Maps to a host URL, optional API key, and the provider's model ID.
 */
interface ModelConfig {
  baseURL: string;
  apiKey?: string;
}

const models: Record<string, ModelConfig> = {
  'qwen/qwen3.5-9b': {
    baseURL: 'http://192.168.50.144:1234/v1',
  },
  'zai-org/glm-4.6v-flash': {
    baseURL: 'http://192.168.50.144:1234/v1',
  },
};

// Cache providers by baseURL so we reuse one per host
const providerCache = new Map<string, ReturnType<typeof createOpenAICompatible>>();

function getProvider(baseURL: string, apiKey?: string) {
  const cacheKey = `${baseURL}::${apiKey ?? ''}`;
  let provider = providerCache.get(cacheKey);
  if (!provider) {
    provider = createOpenAICompatible({
      name: new URL(baseURL).hostname,
      baseURL,
      apiKey,
    });
    providerCache.set(cacheKey, provider);
  }
  return provider;
}

/**
 * Resolve a model name to a LanguageModelV3 instance via OpenAICompatible.
 */
export function resolveModel(name: string): LanguageModelV3 {
  const entry = models[name];
  if (!entry) throw new Error(`Unknown model: "${name}". Available: ${Object.keys(models).join(', ')}`);
  const provider = getProvider(entry.baseURL, entry.apiKey);
  return provider(name);
}
