import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Badge } from './Badge.vue'

export const badgeVariants = cva(
  'inline-flex w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-xs font-medium whitespace-nowrap '
  + 'classic:font-mono classic:uppercase classic:tracking-caps',
  {
    variants: {
      variant: {
        neutral: 'bg-control text-fg-muted',
        accent: 'bg-accent/15 text-accent',
        success: 'bg-success/15 text-success',
        warning: 'bg-warning/15 text-warning',
        danger: 'bg-danger/15 text-danger',
        solid: 'bg-accent text-on-accent',
        outline: 'text-fg-faint ring-1 ring-inset ring-line-strong',
      },
      size: {
        sm: 'h-5 px-1.5 text-micro',
        md: 'h-6 px-2 text-caption',
      },
    },
    defaultVariants: {
      variant: 'neutral',
      size: 'sm',
    },
  },
)

export type BadgeVariants = VariantProps<typeof badgeVariants>
