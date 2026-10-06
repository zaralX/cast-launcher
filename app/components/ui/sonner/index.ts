export { default as Toaster } from './Sonner.vue'
export { default as Toast } from './Toast.vue'

export type ToastColor = 'neutral' | 'info' | 'success' | 'warning' | 'error'

export interface ToastAction {
  label: string
  onClick: (event: MouseEvent) => void
}
