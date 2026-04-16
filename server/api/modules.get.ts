import { getAllModules } from '#server/gameplay/gameplayModule';

export default defineEventHandler(() => {
  const modules = getAllModules();
  return {
    modules: Array.from(modules.entries()).map(([type, mod]) => ({
      type,
    })),
  };
});
