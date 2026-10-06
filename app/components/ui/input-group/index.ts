import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as InputGroup } from './InputGroup.vue'
export { default as InputGroupAddon } from './InputGroupAddon.vue'
export { default as InputGroupInput } from './InputGroupInput.vue'
export { default as InputGroupText } from './InputGroupText.vue'

export const inputGroupAddonVariants = cva(
  'flex h-full cursor-text items-center gap-2 text-fg-faint select-none',
  {
    variants: {
      align: {
        'inline-start': 'order-first pl-2.5',
        'inline-end': 'order-last pr-2.5',
      },
    },
    defaultVariants: {
      align: 'inline-start',
    },
  },
)

export type InputGroupVariants = VariantProps<typeof inputGroupAddonVariants>
