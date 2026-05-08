<template>
  <aside class="debug-panel">
    <div class="debug-header">
      <span class="debug-title">Debug</span>
    </div>
    <div class="debug-body">
      <Collapsible>
        <template #header>Prompt</template>
        <TreeView v-if="promptDebug" :data="promptDebug" />
        <span v-else class="debug-empty">No prompt data yet</span>
      </Collapsible>
      <Collapsible>
        <template #header>Thoughts</template>
        <pre v-if="thoughts">{{ thoughts }}</pre>
        <span v-else class="debug-empty">No thoughts yet</span>
      </Collapsible>
      <Collapsible>
        <template #header>Token Usage</template>
        <table v-if="tokenUsage">
          <tr><th>Prompt</th><td>{{ tokenUsage.prompt_tokens }}</td></tr>
          <tr><th>Completion</th><td>{{ tokenUsage.completion_tokens }}</td></tr>
          <tr class="usage-total"><th>Total</th><td>{{ tokenUsage.total_tokens }}</td></tr>
        </table>
        <span v-else class="debug-empty">No usage data yet</span>
      </Collapsible>
    </div>
  </aside>
</template>

<script setup lang="ts">
import type { LlmUsage } from '~/composables/useLlmStream';
import type { PromptDebug } from '~/composables/useGame';
import Collapsible from '~/components/Collapsible.vue';
import TreeView from '~/components/TreeView.vue';

defineProps<{
  thoughts?: string;
  tokenUsage?: LlmUsage;
  promptDebug?: PromptDebug;
}>();
</script>

<style scoped>
.debug-panel {
  inline-size: var(--size-content-2);
  flex-shrink: 0;
  background: var(--surface-2);
  border-inline-start: var(--border-size-1) solid var(--surface-3);
  display: flex;
  flex-direction: column;
  font-size: var(--font-size-0);
}

.debug-header {
  display: flex;
  align-items: center;
  padding: var(--size-3) var(--size-4);
  border-block-end: var(--border-size-1) solid var(--surface-3);
}

.debug-title {
  font-weight: var(--font-weight-7);
  font-size: var(--font-size-1);
  text-transform: uppercase;
  letter-spacing: var(--font-letterspacing-3);
  color: var(--text-2);
}

.debug-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--size-3);
  display: flex;
  flex-direction: column;
  gap: var(--size-2);
}

.debug-empty {
  color: var(--text-3);
  font-style: italic;
}

.usage-total {
  border-block-start: var(--border-size-1) solid var(--surface-3);
}

.usage-total td {
  font-weight: var(--font-weight-6);
  text-align: end;
  font-family: var(--font-mono, monospace);
  font-variant-numeric: tabular-nums;
}

:deep(.collapsible .collapsible-header) {
  padding: var(--size-1);
}
</style>
