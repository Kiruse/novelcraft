<template>
  <div class="tree-node">
    <div
      class="tree-row"
      :class="{ 'tree-row--expandable': isExpandable }"
      @click="isExpandable && (expanded = !expanded)"
    >
      <span class="tree-caret">{{ isExpandable ? (expanded ? '▾' : '▸') : '·' }}</span>
      <span v-if="label" class="tree-key">{{ label ? label : depth === 0 ? 'root' : 'node' }}</span>
      <span v-if="isExpandable && !expanded" class="tree-hint">{{ summary }}</span>
      <span v-else-if="!isExpandable" class="tree-val">{{ displayValue }}</span>
    </div>
    <div v-if="expanded" class="tree-children">
      <template v-if="isArr">
        <TreeView
          v-for="(item, i) in data"
          :key="i"
          :label="`${i}: ${shortObjStr(item)}`"
          :data="item"
          :depth="depth + 1"
        />
      </template>
      <template v-else-if="isLongStr">
        <pre class="tree-str-full">{{ data }}</pre>
      </template>
      <template v-else>
        <TreeView
          v-for="[key, val] in entries"
          :key="key"
          :label="key"
          :data="val"
          :depth="depth + 1"
        />
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
const STR_TRUNCATE = 60;

const props = withDefaults(defineProps<{
  data: unknown;
  label?: string;
  depth?: number;
}>(), {
  label: undefined,
  depth: 0,
});

const isArr = computed(() => Array.isArray(props.data));
const isObj = computed(() => props.data !== null && typeof props.data === 'object' && !isArr.value);
const isLongStr = computed(() => typeof props.data === 'string' && props.data.length > STR_TRUNCATE);
const isExpandable = computed(() => isArr.value || isObj.value || isLongStr.value);

const expanded = ref(props.depth < 1);

const entries = computed(() => {
  if (!isObj.value) return [];
  return Object.entries(props.data as Record<string, unknown>);
});

const displayValue = computed(() => {
  const v = props.data;
  if (v === null) return 'null';
  if (v === undefined) return 'undefined';
  if (typeof v === 'string') return `"${v}"`;
  return String(v);
});

const summary = computed(() => {
  if (isArr.value) return `[${(props.data as unknown[]).length}]`;
  if (isObj.value) return `{${entries.value.length}}`;
  if (isLongStr.value) return `"${(props.data as string).slice(0, STR_TRUNCATE)}…"`;
  return '';
});

function shortObjStr(obj: any): string {
  const simpleObj = Object.fromEntries(Object.entries(obj).map(([key, value]) =>
    typeof value !== 'object' && value !== undefined
      ? [key, value + '']
      : undefined
  ).filter(Boolean) as [string, any])
  return JSON.stringify(simpleObj);
}
</script>

<style scoped>
.tree-node {
  font-family: var(--font-mono, monospace);
  font-size: var(--font-size-0);
  line-height: var(--font-lineheight-4);
}

.tree-row {
  display: flex;
  gap: var(--size-1);
  align-items: baseline;
  padding: 1px var(--size-1);
  border-radius: var(--radius-1);
}

.tree-row--expandable {
  cursor: pointer;
}

.tree-row--expandable:hover {
  background: var(--surface-2);
}

.tree-caret {
  display: inline-block;
  inline-size: 1em;
  text-align: center;
  color: var(--text-4);
  flex-shrink: 0;
  user-select: none;
}

.tree-key {
  color: var(--indigo-4);
  white-space: nowrap;
}

.tree-hint {
  color: var(--text-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tree-val {
  color: var(--text-2);
  word-break: break-word;
}

.tree-children {
  padding-inline-start: var(--size-4);
}

.tree-str-full {
  color: var(--text-2);
}
</style>
