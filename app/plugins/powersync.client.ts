import { initLocalDb } from '~/composables/useLocalDb';

export default defineNuxtPlugin(async () => {
  if (import.meta.client) {
    await initLocalDb();
  }
});
