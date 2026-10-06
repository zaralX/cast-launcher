import type { ToastAction, ToastColor } from '@/components/ui/sonner'
import { toast } from 'vue-sonner'
import { Toast } from '@/components/ui/sonner'

export interface AppToast {
  title: string
  description?: string
  icon?: string
  color?: ToastColor
  duration?: number
  actions?: ToastAction[]
}

export function useAppToast() {
  function add(options: AppToast) {
    const duration = options.duration ?? 5000
    toast.custom(markRaw(Toast), { duration, componentProps: { ...options, duration } })
  }

  return { add }
}
