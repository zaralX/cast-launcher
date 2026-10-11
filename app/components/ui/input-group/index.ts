import type { VariantProps } from 'class-variance-authority'
import type { InjectionKey, Ref } from 'vue'
import { cva } from 'class-variance-authority'

export { default as InputGroup } from './InputGroup.vue'
export { default as InputGroupAddon } from './InputGroupAddon.vue'
export { default as InputGroupButton } from './InputGroupButton.vue'
export { default as InputGroupInput } from './InputGroupInput.vue'
export { default as InputGroupText } from './InputGroupText.vue'

export const inputGroupVariants = cva(
  'group/input-group relative flex w-full min-w-0 items-center rounded-sm bg-control text-fg transition-[box-shadow] duration-150 '
  + 'has-[[data-slot=input-group-control]:focus-visible]:shadow-focus '
  + 'has-[[data-slot=input-group-control][aria-invalid=true]]:shadow-invalid '
  + 'has-[[data-slot=input-group-control]:disabled]:opacity-45',
  {
    variants: {
      size: {
        sm: 'h-7 text-caption',
        md: 'h-8 text-body',
        lg: 'h-10 text-title',
      },
    },
    defaultVariants: {
      size: 'md',
    },
  },
)

export const inputGroupAddonVariants = cva(
  'flex h-full shrink-0 cursor-text items-center gap-2 text-fg-faint select-none',
  {
    variants: {
      align: {
        'inline-start': 'order-first pl-2.5 has-[>button]:pl-0.5',
        'inline-end': 'order-last pr-2.5 has-[>button]:pr-0.5',
      },
    },
    defaultVariants: {
      align: 'inline-start',
    },
  },
)

export type InputGroupVariants = VariantProps<typeof inputGroupVariants>
export type InputGroupAddonVariants = VariantProps<typeof inputGroupAddonVariants>

export const INPUT_GROUP_SIZE: InjectionKey<Ref<InputGroupVariants['size']>> = Symbol('input-group-size')
