const open = ref(false);

export function useShortcutsDialog() {
  function show() {
    open.value = true;
  }

  function close() {
    open.value = false;
  }

  function toggle() {
    open.value = !open.value;
  }

  return { open: readonly(open), show, close, toggle };
}
