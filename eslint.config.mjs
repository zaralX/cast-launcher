// @ts-check
import withNuxt from './.nuxt/eslint.config.mjs'

export default withNuxt(
  { ignores: ['src-tauri/**'] },
  {
    rules: {
      'no-restricted-imports': ['error', {
        paths: [
          {
            name: '@tauri-apps/api/core',
            importNames: ['invoke'],
            message: 'Команды Rust вызываются через call() из utils/backend.ts.',
          },
          {
            name: '@tauri-apps/api/event',
            importNames: ['listen', 'once', 'emit'],
            message: 'События лаунчера приходят через onLauncherEvent() из utils/backend.ts.',
          },
        ],
      }],
    },
  },
  {
    // telemetry зовёт команду плагина aptabase, её нет в Commands
    files: ['app/utils/backend.ts', 'app/utils/telemetry.ts'],
    rules: {
      'no-restricted-imports': 'off',
    },
  },
)
