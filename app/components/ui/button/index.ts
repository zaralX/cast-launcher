import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'
import { FOCUS_RING } from '@/lib/styles'

export { default as Button } from './Button.vue'

// One primary button per screen; the save bar uses strong so it does not compete with Play.
export const buttonVariants = cva(
  `${FOCUS_RING} relative inline-flex shrink-0 cursor-pointer items-center justify-center whitespace-nowrap rounded-sm font-medium `
  + 'transition-[background-color,color,filter,border-color] duration-150 '
  + 'disabled:pointer-events-none disabled:opacity-45 aria-disabled:pointer-events-none aria-disabled:opacity-45 '
  + 'classic:font-mono classic:text-micro classic:uppercase classic:tracking-caps',
  {
    variants: {
      variant: {
        'primary': 'bg-accent text-on-accent hover:brightness-110',
        'secondary': 'bg-control text-fg hover:bg-hover aria-expanded:bg-hover',
        'ghost': 'text-fg-muted hover:bg-control hover:text-fg aria-expanded:bg-control aria-expanded:text-fg aria-pressed:bg-control aria-pressed:text-fg',
        'strong': 'bg-fg text-sheet hover:brightness-90',
        'danger': 'bg-danger text-on-danger hover:brightness-110',
        'danger-ghost': 'text-danger hover:bg-danger/12',
        'dashed': 'border border-dashed border-line-strong text-fg-muted hover:border-accent hover:text-accent',
        'link': 'text-accent underline-offset-2 hover:underline',
        'play': 'bg-control text-fg hover:bg-accent hover:text-on-accent group-hover/card:bg-accent group-hover/card:text-on-accent',
      },
      size: {
        'sm': 'h-7 gap-1.5 px-2.5 text-caption',
        'md': 'h-8 gap-2 px-3 text-body',
        'lg': 'h-10 gap-2 px-4.5 text-title',
        'icon-sm': 'size-7',
        'icon': 'size-8',
        'icon-lg': 'size-10',
        'round': 'size-9 rounded-pill',
      },
    },
    compoundVariants: [
      {
        variant: 'link',
        size: ['sm', 'md', 'lg'],
        class: 'h-auto px-0',
      },
    ],
    defaultVariants: {
      variant: 'secondary',
      size: 'md',
    },
  },
)

export type ButtonVariants = VariantProps<typeof buttonVariants>
