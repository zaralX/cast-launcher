import type { VariantProps } from 'class-variance-authority'
import type { InjectionKey, Ref } from 'vue'
import { cva } from 'class-variance-authority'
import { FOCUS_RING } from '@/lib/styles'

export { default as RadioGroup } from './RadioGroup.vue'
export { default as RadioGroupItem } from './RadioGroupItem.vue'

export const radioGroupVariants = cva('', {
  variants: {
    variant: {
      segmented: 'flex gap-0.5 rounded-md bg-control p-0.75',
      tiles: 'flex flex-wrap gap-2',
      cards: 'grid gap-3',
      bare: 'flex flex-wrap gap-3',
    },
  },
  defaultVariants: {
    variant: 'segmented',
  },
})

export const radioGroupItemVariants = cva(
  `${FOCUS_RING} group relative flex cursor-pointer items-center justify-center gap-2 transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-45`,
  {
    variants: {
      variant: {
        segmented: 'min-h-6.5 flex-1 rounded-sm px-2.5 text-caption font-medium text-fg-muted hover:text-fg data-[state=checked]:bg-raise data-[state=checked]:text-fg',
        tiles: 'min-h-8 rounded-sm bg-control px-3 text-fg-muted hover:bg-hover hover:text-fg data-[state=checked]:bg-accent/15 data-[state=checked]:text-accent',
        cards: 'justify-start gap-4 rounded-md bg-panel p-4 text-left ring-1 ring-card-line ring-inset hover:ring-line-strong data-[state=checked]:ring-2 data-[state=checked]:ring-accent',
        bare: '',
      },
    },
    defaultVariants: {
      variant: 'segmented',
    },
  },
)

export type RadioGroupVariants = VariantProps<typeof radioGroupVariants>

export const RADIO_GROUP_VARIANT: InjectionKey<Ref<RadioGroupVariants['variant']>> = Symbol('radio-group-variant')
