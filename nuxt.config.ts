// https://nuxt.com/docs/api/configuration/nuxt-config
import tailwindcss from "@tailwindcss/vite";

export default defineNuxtConfig({
  compatibilityDate: '2025-07-15',
  devtools: { enabled: true },
  css: ['~/assets/css/main.css'],
  ssr: false,
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
  fonts: {
    provider: 'google',
    families: [
      { name: 'Golos Text', provider: 'google', weights: [400, 500, 600, 700] },
      { name: 'Unbounded', provider: 'google', weights: [400, 600, 700, 800] },
      { name: 'JetBrains Mono', provider: 'google', weights: [400, 500, 600] },
    ]
  },
  app: {
    layoutTransition: { name: 'layout', mode: 'out-in' },
    pageTransition: { name: 'page', mode: 'out-in' },
  },
  ignore: ['**/src-tauri/**'],
  modules: ['@nuxt/image', '@pinia/nuxt', '@nuxt/ui']
})
