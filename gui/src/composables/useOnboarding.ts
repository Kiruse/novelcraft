import { db, localOnboarding } from '~/db';

const completed = ref(false);
let initialized = false;

export function useOnboarding() {
  if (!initialized) {
    initialized = true;
    db.select({ completed: localOnboarding.completed })
      .from(localOnboarding)
      .then((rows) => {
        completed.value = rows.length > 0 && rows[0].completed === 1;
      })
      .catch(() => {
        completed.value = false;
      });
  }

  async function complete() {
    await db.delete(localOnboarding);
    await db.insert(localOnboarding).values({ completed: 1 });
    completed.value = true;
  }

  async function reset() {
    await db.delete(localOnboarding);
    completed.value = false;
  }

  return { completed: readonly(completed), complete, reset };
}
