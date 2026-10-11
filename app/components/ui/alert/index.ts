import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Alert } from './Alert.vue'
export { default as AlertDescription } from './AlertDescription.vue'
export { default as AlertTitle } from './AlertTitle.vue'

export const alertVariants = cva(
  'relative flex w-full items-start gap-3 rounded-md px-4 py-3 text-body text-fg-muted ring-1 ring-inset',
  {
    variants: {
      variant: {
        neutral: 'bg-panel ring-card-line',
        accent: 'bg-accent/10 ring-accent/25',
        warning: 'bg-warning/10 ring-warning/25',
        danger: 'bg-danger/10 ring-danger/25',
      },
    },
    defaultVariants: {
      variant: 'neutral',
    },
  },
)

export const alertIconVariants = cva('mt-0.5 size-4 shrink-0', {
  variants: {
    variant: {
      neutral: 'text-fg-faint',
      accent: 'text-accent',
      warning: 'text-warning',
      danger: 'text-danger',
    },
  },
  defaultVariants: {
    variant: 'neutral',
  },
})

export type AlertVariants = VariantProps<typeof alertVariants>
