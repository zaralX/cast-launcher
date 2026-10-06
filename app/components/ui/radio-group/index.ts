import type { VariantProps } from 'class-variance-authority'
import type { InjectionKey, Ref } from 'vue'
import { cva } from 'class-variance-authority'

export { default as RadioGroup } from './RadioGroup.vue'
export { default as RadioGroupItem } from './RadioGroupItem.vue'

export const radioGroupVariants = cva('', {
  variants: {
    variant: {
      segmented: 'flex border border-line',
      tiles: 'flex flex-wrap gap-2',
      cards: 'grid gap-3',
    },
  },
  defaultVariants: {
    variant: 'segmented',
  },
})

export const radioGroupItemVariants = cva(
  'group relative flex cursor-pointer items-center justify-center gap-2 outline-none transition-colors duration-300 focus-visible:ring-1 focus-visible:ring-acid/60 focus-visible:ring-inset disabled:cursor-not-allowed disabled:opacity-40',
  {
    variants: {
      variant: {
        segmented: 'flex-1 border-l border-line text-fg-faint first:border-l-0 hover:bg-ink-700/50 hover:text-fg-muted data-[state=checked]:bg-ink-700 data-[state=checked]:text-fg',
        tiles: 'border border-line text-fg-muted hover:border-line-strong hover:text-fg data-[state=checked]:border-fg data-[state=checked]:text-fg',
        cards: 'justify-start gap-4 border border-line p-4 text-left hover:border-line-strong hover:bg-ink-700 data-[state=checked]:border-acid/60 data-[state=checked]:bg-ink-700',
      },
    },
    defaultVariants: {
      variant: 'segmented',
    },
  },
)

export type RadioGroupVariants = VariantProps<typeof radioGroupVariants>

export const RADIO_GROUP_VARIANT: InjectionKey<Ref<RadioGroupVariants['variant']>> = Symbol('radio-group-variant')
