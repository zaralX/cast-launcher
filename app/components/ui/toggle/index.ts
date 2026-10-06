import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Toggle } from './Toggle.vue'

export const toggleVariants = cva(
  'inline-flex cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap outline-none transition-colors duration-300 focus-visible:ring-1 focus-visible:ring-acid/60 disabled:pointer-events-none disabled:opacity-75',
  {
    variants: {
      variant: {
        chip: 'border border-line font-mono uppercase tracking-caps text-fg-faint hover:border-line-strong hover:text-fg-muted data-[state=on]:border-acid data-[state=on]:text-acid',
      },
      size: {
        sm: 'px-2.5 py-1 text-label',
      },
    },
    defaultVariants: {
      variant: 'chip',
      size: 'sm',
    },
  },
)

export type ToggleVariants = VariantProps<typeof toggleVariants>
