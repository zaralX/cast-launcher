import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Button } from './Button.vue'

const SWEEP = 'isolate overflow-hidden before:absolute before:inset-0 before:-z-10 before:origin-left before:scale-x-0 '
  + 'before:transition-transform before:duration-500 before:ease-deck hover:before:scale-x-100'

export const buttonVariants = cva(
  'relative inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 whitespace-nowrap font-mono uppercase tracking-caps outline-none transition-all duration-300 focus-visible:ring-1 focus-visible:ring-acid/60 disabled:pointer-events-none disabled:opacity-75 aria-disabled:pointer-events-none aria-disabled:opacity-75',
  {
    variants: {
      variant: {
        'fill': `${SWEEP} border border-line text-fg before:bg-acid hover:border-acid hover:text-on-acid`,
        'danger': `${SWEEP} border border-danger/30 text-danger before:bg-danger-strong hover:border-danger-strong hover:text-on-danger`,
        'outline': 'border border-line text-fg-muted hover:border-line-strong hover:text-fg',
        'dashed': 'border border-dashed border-line text-fg-faint hover:border-acid/50 hover:text-acid',
        'ghost': 'text-fg-faint hover:bg-ink-600 hover:text-fg',
        'toolbar': 'text-fg-faint hover:text-fg aria-pressed:text-acid',
        'quiet': 'text-fg-muted hover:text-acid',
        'quiet-danger': 'text-fg-muted hover:text-danger',
        'link': 'text-acid hover:opacity-70',
      },
      size: {
        'xs': 'h-7 gap-1.5 px-3 text-label',
        'sm': 'h-8 px-3 text-label',
        'md': 'h-9 px-3.5 text-label',
        'lg': 'h-10 px-6 text-caption',
        'xl': 'h-11 px-6 text-caption',
        'icon-sm': 'size-8',
        'icon': 'size-9',
        'icon-lg': 'size-11',
      },
    },
    compoundVariants: [
      {
        variant: ['quiet', 'quiet-danger', 'link'],
        size: ['xs', 'sm', 'md', 'lg', 'xl'],
        class: 'h-auto px-0 py-2',
      },
    ],
    defaultVariants: {
      variant: 'fill',
      size: 'md',
    },
  },
)

export type ButtonVariants = VariantProps<typeof buttonVariants>
