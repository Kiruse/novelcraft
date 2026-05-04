import type { Plugin } from 'vite';

const VUE_IMPORTS: Record<string, [string, string]> = {
  ref: ['vue', 'ref'],
  reactive: ['vue', 'reactive'],
  computed: ['vue', 'computed'],
  watch: ['vue', 'watch'],
  readonly: ['vue', 'readonly'],
  onMounted: ['vue', 'onMounted'],
  onUnmounted: ['vue', 'onUnmounted'],
  nextTick: ['vue', 'nextTick'],
  useRoute: ['vue-router', 'useRoute'],
  useRouter: ['vue-router', 'useRouter'],
};

function collectUsedIdentifiers(code: string): string[] {
  const used: string[] = [];
  for (const name of Object.keys(VUE_IMPORTS)) {
    const regex = new RegExp(`(?:^|[^.\\w$])${name}(?:\\s|\\(|$|[^\\w$])`, 'm');
    if (regex.test(code)) {
      used.push(name);
    }
  }
  return used;
}

function buildImportStmt(identifiers: string[]): string | null {
  if (identifiers.length === 0) return null;

  const bySource = new Map<string, string[]>();
  for (const id of identifiers) {
    const [source] = VUE_IMPORTS[id];
    if (!bySource.has(source)) bySource.set(source, []);
    bySource.get(source)!.push(id);
  }

  const parts: string[] = [];
  for (const [source, ids] of bySource) {
    parts.push(`import { ${ids.join(', ')} } from '${source}';`);
  }
  return parts.join('\n');
}

function skipWhitespaceAndComments(code: string, pos: number): number {
  const len = code.length;
  while (pos < len) {
    if (code[pos] === '\n' || code[pos] === '\r' || code[pos] === ' ' || code[pos] === '\t') {
      pos++;
    } else if (code.startsWith('//', pos)) {
      const nl = code.indexOf('\n', pos);
      pos = nl === -1 ? len : nl + 1;
    } else if (code.startsWith('/*', pos)) {
      const close = code.indexOf('*/', pos);
      pos = close === -1 ? len : close + 2;
    } else {
      break;
    }
  }
  return pos;
}

function isStringDirective(code: string, pos: number): boolean {
  const rest = code.slice(pos);
  const match = rest.match(/^(['"])(use \w+)\1\s*;?\s*\n/);
  return match !== null;
}

function skipStringDirective(code: string, pos: number): number {
  const match = code.slice(pos).match(/^(['"])(use \w+)\1\s*;?\s*\n/);
  if (!match) return pos;
  return pos + match[0].length;
}

function findFirstImportStart(code: string): number | null {
  let pos = 0;
  const len = code.length;

  while (pos < len) {
    pos = skipWhitespaceAndComments(code, pos);

    if (pos >= len) break;

    if (isStringDirective(code, pos)) {
      pos = skipStringDirective(code, pos);
      continue;
    }

    if (code.startsWith('import ', pos) || code.startsWith('import\t', pos) || code.startsWith('import\n', pos)) {
      return pos;
    }

    if (code.startsWith('export ', pos)) {
      const after = code.slice(pos + 7);
      if (after.startsWith('* from ') || after.startsWith('{')) {
        return pos;
      }
      pos += 7;
      continue;
    }

    break;
  }

  return null;
}

function skipImportStatement(code: string, start: number): number {
  let i = start;
  const len = code.length;
  let inString: string | null = null;
  let braceDepth = 0;
  let parenDepth = 0;

  while (i < len) {
    const ch = code[i];

    if (inString) {
      if (ch === '\\') { i += 2; continue; }
      if (ch === inString) inString = null;
      i++;
      continue;
    }

    if (ch === '"' || ch === "'" || ch === '`') {
      inString = ch;
      i++;
      continue;
    }

    if (ch === '{') braceDepth++;
    else if (ch === '}') braceDepth--;
    else if (ch === '(') parenDepth++;
    else if (ch === ')') parenDepth--;

    if (ch === ';') {
      return i + 1;
    }

    if (ch === '\n' && braceDepth <= 0 && parenDepth <= 0) {
      return i;
    }

    i++;
  }

  return len;
}

function injectImports(code: string, importStmt: string): string {
  const insertPos = findFirstImportStart(code);
  if (insertPos === null) {
    return importStmt + '\n' + code;
  }

  return code.slice(0, insertPos) + importStmt + '\n' + code.slice(insertPos);
}

interface ScriptBlock {
  attrs: string;
  content: string;
  contentStart: number;
  contentEnd: number;
}

function extractScriptBlocks(source: string): ScriptBlock[] {
  const blocks: ScriptBlock[] = [];
  const regex = /<(script)\b([^>]*)>([\s\S]*?)<\/\1>/g;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(source)) !== null) {
    blocks.push({
      attrs: match[2],
      content: match[3],
      contentStart: match.index + match[0].indexOf('>') + 1,
      contentEnd: match.index + match[0].lastIndexOf('<'),
    });
  }

  return blocks;
}

function shouldProcessTs(id: string): boolean {
  return id.endsWith('.ts') && !id.endsWith('.d.ts');
}

function shouldProcessVue(id: string): boolean {
  return id.endsWith('.vue');
}

function transformCode(code: string): { code: string; map: null } | null {
  const used = collectUsedIdentifiers(code);
  if (used.length === 0) return null;

  const stmt = buildImportStmt(used);
  if (!stmt) return null;

  const result = injectImports(code, stmt);
  if (result === code) return null;

  return { code: result, map: null };
}

export function autoImport(): Plugin {
  return {
    name: 'novelcraft:auto-import',
    enforce: 'pre',
    transform(code, id) {
      if (shouldProcessTs(id)) {
        return transformCode(code);
      }

      if (shouldProcessVue(id)) {
        return transformVue(code);
      }

      return null;
    },
    async handleHotUpdate({ file, server, modules, read }) {
      if (!shouldProcessTs(file) && !shouldProcessVue(file)) return;

      const code = await read();
      const used = collectUsedIdentifiers(code);
      if (used.length === 0) return;

      const stmt = buildImportStmt(used);
      if (!stmt) return;

      const result = shouldProcessTs(file)
        ? transformCode(code)?.code ?? code
        : transformVue(code)?.code ?? code;

      if (result === code) return;

      for (const mod of modules) {
        if (mod.id === file) {
          server.moduleGraph.invalidateModule(mod);
          break;
        }
      }
    },
  };
}

function transformVue(source: string): { code: string; map: null } | null {
  const blocks = extractScriptBlocks(source);
  if (blocks.length === 0) return null;

  let modified = false;
  let result = source;

  for (let i = blocks.length - 1; i >= 0; i--) {
    const block = blocks[i];
    if (!block.attrs.includes('setup')) continue;

    const transformed = transformCode(block.content);
    if (!transformed) continue;

    result =
      result.slice(0, block.contentStart) +
      transformed.code +
      result.slice(block.contentEnd);

    modified = true;
  }

  if (!modified) return null;
  return { code: result, map: null };
}
