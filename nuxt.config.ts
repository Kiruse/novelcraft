// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: true },

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
