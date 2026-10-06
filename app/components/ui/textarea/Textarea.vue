<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { useVModel } from '@vueuse/core'
import { FIELD } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
  defaultValue?: string | number
  modelValue?: string | number
  autoresize?: boolean
}>()

const emits = defineEmits<{
  (e: 'update:modelValue', payload: string | number): void
}>()

const modelValue = useVModel(props, 'modelValue', emits, {
  passive: true,
  defaultValue: props.defaultValue,
})

const element = ref<HTMLTextAreaElement | null>(null)

// field-sizing is not in WebKit, which Tauri uses on macOS and Linux.
function resize() {
  const textarea = element.value
  if (!props.autoresize || !textarea) return

  textarea.style.height = 'auto'
  textarea.style.height = `${textarea.scrollHeight}px`
}

watch(modelValue, () => nextTick(resize))
onMounted(resize)
</script>

<template>
  <textarea
    ref="element"
    v-model="modelValue"
    data-slot="textarea"
    :class="cn(FIELD, 'block w-full resize-none px-2.5 py-1.5', props.class)"
  />
</template>
