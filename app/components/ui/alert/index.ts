import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Alert } from './Alert.vue'
export { default as AlertDescription } from './AlertDescription.vue'
export { default as AlertTitle } from './AlertTitle.vue'

export const alertVariants = cva(
  'relative flex w-full items-start gap-2.5 border bg-ink-900 px-4 py-3 text-body leading-relaxed text-fg-muted',
  {
    variants: {
      variant: {
        neutral: 'border-line',
        accent: 'border-acid/40',
        warning: 'border-warning/30',
        danger: 'border-danger/30',
      },
    },
    defaultVariants: {
      variant: 'neutral',
    },
  },
)

export const alertIconVariants = cva('mt-0.5 size-3.5 shrink-0', {
  variants: {
    variant: {
      neutral: 'text-fg-faint',
      accent: 'text-acid',
      warning: 'text-warning',
      danger: 'text-danger',
    },
  },
  defaultVariants: {
    variant: 'neutral',
  },
})

export type AlertVariants = VariantProps<typeof alertVariants>
