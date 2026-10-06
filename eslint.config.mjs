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
            message: 'Call Rust commands through call() from utils/backend.ts.',
          },
          {
            name: '@tauri-apps/api/event',
            importNames: ['listen', 'once', 'emit'],
            message: 'Listen to launcher events through onLauncherEvent() from utils/backend.ts.',
          },
        ],
      }],
    },
  },
  {
    // shadcn-vue primitives come from the CLI, a later `add` would undo hand fixes of its `any`s
    files: ['app/components/ui/**'],
    rules: {
      '@typescript-eslint/no-explicit-any': 'off',
      'vue/require-default-prop': 'off',
    },
  },
  {
    // telemetry calls the aptabase plugin command, which is not in Commands
    files: ['app/utils/backend.ts', 'app/utils/telemetry.ts'],
    rules: {
      'no-restricted-imports': 'off',
    },
  },
)
