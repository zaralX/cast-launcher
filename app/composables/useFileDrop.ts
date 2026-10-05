import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { UnlistenFn } from '@tauri-apps/api/event'

export function useFileDrop(onDrop: (paths: string[]) => unknown) {
  const hovering = ref(false)

  let unlisten: UnlistenFn | undefined
  let disposed = false

  onMounted(async () => {
    const stop = await getCurrentWebview().onDragDropEvent(({ payload }) => {
      hovering.value = payload.type === 'enter' || payload.type === 'over'
      if (payload.type === 'drop') onDrop(payload.paths)
    })

    // компонент мог размонтироваться, пока регистрировался слушатель
    if (disposed) stop()
    else unlisten = stop
  })

  onBeforeUnmount(() => {
    disposed = true
    unlisten?.()
  })

  return hovering
}
