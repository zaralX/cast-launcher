<script setup lang="ts" generic="T extends string | number">
import type { HTMLAttributes } from 'vue'
import type { SelectOption } from '~/types/ui'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const props = defineProps<{
  items: readonly (T | SelectOption<T>)[]
  placeholder?: string
  disabled?: boolean
  size?: 'md' | 'lg'
  class?: HTMLAttributes['class']
}>()

const model = defineModel<T>()

const options = computed<SelectOption<T>[]>(() => props.items.map(item =>
  typeof item === 'object' ? item : { label: String(item), value: item },
))
</script>

<template>
  <Select
    v-model="model"
    :disabled="disabled"
  >
    <SelectTrigger
      :size="size"
      :class="props.class"
    >
      <SelectValue :placeholder="placeholder" />
    </SelectTrigger>

    <SelectContent>
      <SelectItem
        v-for="option in options"
        :key="option.value"
        :value="option.value"
        :disabled="option.disabled"
      >
        <span class="flex items-center gap-1.5">
          <Icon
            v-if="option.icon"
            :name="option.icon"
            mode="svg"
            class="size-4 shrink-0"
          />
          {{ option.label }}
        </span>
      </SelectItem>
    </SelectContent>
  </Select>
</template>
