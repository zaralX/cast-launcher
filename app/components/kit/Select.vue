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

const current = computed(() => options.value.find(option => option.value === model.value) ?? null)
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
      <SelectValue :placeholder="placeholder">
        <span
          v-if="current"
          class="flex min-w-0 items-center gap-1.5"
        >
          <Icon
            v-if="current.icon"
            :name="current.icon"
            mode="svg"
            class="size-4 shrink-0"
          />
          <span class="truncate">{{ current.label }}</span>
        </span>
        <template v-else>
          {{ placeholder }}
        </template>
      </SelectValue>
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
