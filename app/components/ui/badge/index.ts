import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Badge } from './Badge.vue'

export const badgeVariants = cva(
  'inline-flex w-fit shrink-0 items-center justify-center gap-1 overflow-hidden border px-1.5 py-0.5 font-mono text-micro whitespace-nowrap uppercase tracking-caps',
  {
    variants: {
      variant: {
        outline: 'border-line text-fg-muted',
        accent: 'border-acid text-acid',
        warning: 'border-warning/40 text-warning',
        danger: 'border-danger/40 text-danger',
        solid: 'border-acid bg-acid text-on-acid',
      },
    },
    defaultVariants: {
      variant: 'outline',
    },
  },
)

export type BadgeVariants = VariantProps<typeof badgeVariants>
