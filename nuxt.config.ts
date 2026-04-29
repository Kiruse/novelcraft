// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: true },

  css: ['~/assets/css/app.css'],

  vite: {
    optimizeDeps: {
      exclude: ['@powersync/web'],
    },
    worker: {
      format: 'es',
    },
  },

  nitro: {
    typescript: {
      tsConfig: {
        compilerOptions: {
          types: ['@types/bun'],
        },
        include: ['./scripts'],
      },
    },
  },
})
