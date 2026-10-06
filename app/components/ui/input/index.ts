import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'
import { FIELD } from '@/lib/styles'

export { default as Input } from './Input.vue'

export const inputVariants = cva(
  `${FIELD} w-full min-w-0`,
  {
    variants: {
      size: {
        md: 'h-8 px-2.5',
        lg: 'h-9 px-3',
      },
      font: {
        sans: '',
        mono: 'font-mono text-body tabular-nums',
        display: 'font-unbounded text-heading tracking-heading',
      },
    },
    defaultVariants: {
      size: 'md',
      font: 'sans',
    },
  },
)

export type InputVariants = VariantProps<typeof inputVariants>
