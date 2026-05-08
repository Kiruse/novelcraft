const STORAGE_KEY = 'novelcraft:debugMode';

const isDebugMode = ref(localStorage.getItem(STORAGE_KEY) === 'true');

watch(isDebugMode, (val) => {
  try { localStorage.setItem(STORAGE_KEY, String(val)); } catch { /* ignore */ }
});

export function useDebugMode() {
  return { isDebugMode };
}
