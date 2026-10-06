export interface SelectOption<T extends string | number = string> {
  label: string
  value: T
  icon?: string
  disabled?: boolean
}

export type StatusTone = 'muted' | 'accent' | 'warning' | 'danger'
