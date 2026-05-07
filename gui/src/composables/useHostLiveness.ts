import { invoke } from '@tauri-apps/api/core';

interface UnreachableHost {
  url: string;
  error: string;
}

const unreachableHosts = ref<UnreachableHost[]>([]);
const checked = ref(false);

export function useHostLiveness() {
  async function checkHosts() {
    try {
      const result = await invoke<UnreachableHost[]>('ping_hosts');
      unreachableHosts.value = result;
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
  }

  return {
    unreachableHosts: readonly(unreachableHosts),
    checked: readonly(checked),
    checkHosts,
    isHostUnreachable,
    dismiss,
  };
}
