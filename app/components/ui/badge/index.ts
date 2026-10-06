import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Badge } from './Badge.vue'

export const badgeVariants = cva(
  'inline-flex w-fit shrink-0 items-center justify-center gap-1 overflow-hidden border font-mono whitespace-nowrap uppercase tracking-caps',
  {
    variants: {
      variant: {
        outline: 'border-line text-fg-muted',
        accent: 'border-acid text-acid',
        warning: 'border-warning/40 text-warning',
        danger: 'border-danger/40 text-danger',
        solid: 'border-acid bg-acid text-on-acid',
        surface: 'border-line bg-ink-800 text-fg-muted',
      },
      size: {
        sm: 'px-1.5 py-0.5 text-micro',
        md: 'px-3 py-1.5 text-label',
      },
    },
    defaultVariants: {
      variant: 'outline',
      size: 'sm',
    },
  },
)

export type BadgeVariants = VariantProps<typeof badgeVariants>
