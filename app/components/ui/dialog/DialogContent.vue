<script setup lang="ts">
import type { DialogContentEmits, DialogContentProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import { reactiveOmit } from '@vueuse/core'
import {
  DialogClose,
  DialogContent,
  DialogPortal,
  useForwardPropsEmits,
} from 'reka-ui'
import { cn } from '@/lib/utils'
import DialogOverlay from './DialogOverlay.vue'

defineOptions({
  inheritAttrs: false,
})

const props = withDefaults(defineProps<DialogContentProps & {
  class?: HTMLAttributes['class']
  showCloseButton?: boolean
  dismissible?: boolean
}>(), {
  showCloseButton: true,
  dismissible: true,
})
const emits = defineEmits<DialogContentEmits>()

const delegatedProps = reactiveOmit(props, 'class', 'showCloseButton', 'dismissible')

const forwarded = useForwardPropsEmits(delegatedProps, emits)

function guard(event: Event) {
  if (!props.dismissible) event.preventDefault()
}
</script>

<template>
  <DialogPortal>
    <DialogOverlay />
    <DialogContent
      data-slot="dialog-content"
      v-bind="{ ...$attrs, ...forwarded }"
      :class="cn(
        'fixed top-1/2 left-1/2 z-50 flex max-h-[calc(100dvh-4rem)] w-[calc(100vw-2rem)] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden',
        'border border-line bg-ink-800 text-fg shadow-lg outline-none duration-200',
        'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95',
        'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95',
        props.class,
      )"
      @escape-key-down="guard"
      @pointer-down-outside="guard"
      @interact-outside="guard"
    >
      <slot />

      <DialogClose
        v-if="showCloseButton"
        as-child
      >
        <Button
          variant="ghost"
          size="icon-sm"
          icon="i-lucide-x"
          :aria-label="$t('common.close')"
          class="absolute top-4 right-4"
        />
      </DialogClose>
    </DialogContent>
  </DialogPortal>
</template>
