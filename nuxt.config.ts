// https://nuxt.com/docs/api/configuration/nuxt-config
import tailwindcss from '@tailwindcss/vite'

export default defineNuxtConfig({
  modules: ['@nuxt/eslint', '@nuxt/icon', '@nuxt/fonts', '@nuxtjs/color-mode', '@pinia/nuxt', '@nuxt/ui', '@nuxtjs/i18n'],
  ssr: false,
  components: [
    { path: '~/components/ui', pathPrefix: false, extensions: ['vue'] },
    { path: '~/components', pathPrefix: true, ignore: ['**/ui/**'] },
  ],
  devtools: { enabled: true },
  app: {
    layoutTransition: { name: 'layout', mode: 'out-in' },
    pageTransition: { name: 'page', mode: 'out-in' },
  },
  css: ['~/assets/css/main.css'],
  colorMode: {
    classSuffix: '',
    disableTransition: true,
  },
  ignore: ['**/src-tauri/**'],
  compatibilityDate: '2025-07-15',
  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      strictPort: true,
    },
    plugins: [
      tailwindcss(),
    ],
  },
  typescript: {
    tsConfig: {
      vueCompilerOptions: { checkUnknownComponents: true },
    },
  },
  eslint: {
    config: {
      stylistic: true,
    },
  },
  fonts: {
    provider: 'google',
    families: [
      { name: 'Golos Text', provider: 'google', weights: [400, 500, 600, 700] },
      { name: 'Unbounded', provider: 'google', weights: [400, 600, 700, 800] },
      { name: 'JetBrains Mono', provider: 'google', weights: [400, 500, 600] },
    ],
  },
  i18n: {
    vueI18n: 'i18n.config.ts',
    strategy: 'no_prefix',
    defaultLocale: 'ru',
    detectBrowserLanguage: false,
    locales: [
      { code: 'ru', file: 'ru.json' },
      { code: 'en', file: 'en.json' },
    ],
  },
  icon: {
    provider: 'none',
    clientBundle: {
      scan: {
        globInclude: ['app/**/*.{vue,ts}', 'node_modules/@nuxt/ui/dist/**/*.mjs'],
        globExclude: [],
      },
      sizeLimitKb: 0,
    },
  },
})
