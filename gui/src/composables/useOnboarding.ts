import { select, execute } from '~/composables/useLocalDb';

const completed = ref(false);
let initialized = false;

export function useOnboarding() {
  if (!initialized) {
    initialized = true;
    select<{ completed: number }>('SELECT completed FROM local_onboarding').then((rows) => {
      completed.value = rows.length > 0 && rows[0].completed === 1;
    }).catch(() => {
      completed.value = false;
    });
  }

  async function complete() {
    await execute('INSERT OR REPLACE INTO local_onboarding (rowid, completed) VALUES (1, 1)');
    completed.value = true;
  }

  async function reset() {
    await execute('DELETE FROM local_onboarding');
    completed.value = false;
  }

  return { completed: readonly(completed), complete, reset };
}
