// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: true },

  css: ['~/assets/css/app.css'],

  runtimeConfig: {
    public: {
      storyBuilder: process.env.ENABLE_STORY_BUILDER === 'true' || process.env.NODE_ENV === 'development',
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
