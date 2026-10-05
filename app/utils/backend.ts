import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Commands, LauncherEvent } from '~/types/backend'
import { LAUNCHER_EVENT } from '~/types/backend'

type Args<K extends keyof Commands> = Commands[K][0]
type Result<K extends keyof Commands> = Commands[K][1]

export async function call<K extends keyof Commands>(
  ...[command, args]: undefined extends Args<K> ? [K] : [K, Args<K>]
): Promise<Result<K>> {
  try {
    return await invoke<Result<K>>(command, args as Record<string, unknown> | undefined)
  }
  catch (e) {
    throw toLauncherError(e, 'UNKNOWN', { command })
  }
}

export async function onLauncherEvent(handler: (event: LauncherEvent) => void) {
  return await listen<LauncherEvent>(LAUNCHER_EVENT, e => handler(e.payload))
}
