export interface SelectOption<T extends string | number = string> {
  label: string
  value: T
  icon?: string
  disabled?: boolean
}

export type StatusTone = 'muted' | 'accent' | 'success' | 'warning' | 'danger'

export interface TabItem<T extends string = string> {
  value: T
  label: string
  icon?: string
  meta?: string | number
}
