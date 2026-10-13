import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'
import { FOCUS_RING } from '@/lib/styles'

export { default as Toggle } from './Toggle.vue'

export const toggleVariants = cva(
  `${FOCUS_RING} inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-sm font-medium whitespace-nowrap transition-colors duration-150 disabled:pointer-events-none disabled:opacity-45`,
  {
    variants: {
      variant: {
        chip: 'bg-control text-fg-muted hover:bg-hover hover:text-fg data-[state=on]:bg-accent/15 data-[state=on]:text-accent',
        ghost: 'text-fg-faint hover:bg-control hover:text-fg data-[state=on]:bg-control data-[state=on]:text-fg',
      },
      size: {
        'sm': 'h-7 px-2.5 text-caption',
        'icon-sm': 'size-7',
      },
    },
    defaultVariants: {
      variant: 'chip',
      size: 'sm',
    },
  },
)

export type ToggleVariants = VariantProps<typeof toggleVariants>
