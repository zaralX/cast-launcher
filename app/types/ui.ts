export interface SelectOption<T extends string | number = string> {
  label: string
  value: T
  icon?: string
}

export type StatusTone = 'muted' | 'accent' | 'warning' | 'danger'
