import { LazyStore } from '@tauri-apps/plugin-store';

const store = new LazyStore('app.json');
const KEY = 'onboarding_completed';

const completed = ref(false);
let initialized = false;

export function useOnboarding() {
  if (!initialized) {
    initialized = true;
    store
      .get<boolean>(KEY)
      .then((val) => {
        completed.value = val === true;
      })
      .catch(() => {
        completed.value = false;
      });
  }

  async function complete() {
    await store.set(KEY, true);
    completed.value = true;
  }

  async function reset() {
    await store.delete(KEY);
    completed.value = false;
  }

  return { completed: readonly(completed), complete, reset };
}
