import { registerStandardModules } from '#server/gameplay';

export default defineNitroPlugin(() => {
  registerStandardModules();
});
