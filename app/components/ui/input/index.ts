import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'
import { FIELD } from '@/lib/styles'

export { default as Input } from './Input.vue'

export const inputVariants = cva(
  `${FIELD} w-full min-w-0`,
  {
    variants: {
      size: {
        sm: 'h-7 px-2.5 text-caption',
        md: 'h-8 px-2.5',
        lg: 'h-10 px-3 text-title',
      },
      font: {
        sans: '',
        mono: 'font-mono text-caption tabular-nums',
        display: 'font-unbounded text-heading font-semibold tracking-heading',
      },
    },
    defaultVariants: {
      size: 'md',
      font: 'sans',
    },
  },
)

export type InputVariants = VariantProps<typeof inputVariants>
