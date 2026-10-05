import { error as logError } from '@tauri-apps/plugin-log'
import type { LauncherError } from '~/utils/error'

// The log file is read by developers, so texts from Rust are rendered in English.
export function writeErrorLog(error: LauncherError) {
  const summary = error.text
    ? `${uiText(error.text, 'en')} (${error.text.key})`
    : error.message === error.title ? '' : error.message
  const lines = [`[${error.code}] ${summary}`.trim()]

  if (error.details) lines.push(error.details)

  const context = Object.entries(error.context).filter(([, value]) => value !== undefined)
  if (context.length) lines.push(`context: ${JSON.stringify(Object.fromEntries(context))}`)

  logError(lines.join('\n')).catch(() => {})
}
