import { commands, type UnreachableHost } from '~/bindings';
import { unwrap } from '~/utils';

const unreachableHosts = ref<UnreachableHost[]>([]);
const checked = ref(false);

export function useHostLiveness() {
  async function checkHosts() {
    try {
      unreachableHosts.value = await unwrap(commands.pingHosts());
    } catch {
      unreachableHosts.value = [];
    }
    checked.value = true;
  }

  function normalizeUrl(url: string): string {
    return url.replace(/\/+$/, '');
  }

  function isHostUnreachable(baseUrl: string): boolean {
    const target = normalizeUrl(baseUrl);
    return unreachableHosts.value.some((h) => normalizeUrl(h.url) === target);
  }

  function dismiss() {
    unreachableHosts.value = [];
    checked.value = false;
  }

  return {
    unreachableHosts: readonly(unreachableHosts),
    checked: readonly(checked),
    checkHosts,
    isHostUnreachable,
    dismiss,
  };
}
