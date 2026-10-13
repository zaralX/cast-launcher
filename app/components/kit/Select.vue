<script setup lang="ts" generic="T extends string | number">
import type { HTMLAttributes } from 'vue'
import type { SelectOption } from '~/types/ui'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const props = defineProps<{
  items: readonly (T | SelectOption<T>)[]
  placeholder?: string
  disabled?: boolean
  size?: 'sm' | 'md' | 'lg'
  ariaLabel?: string
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
      :aria-label="ariaLabel"
      :class="props.class"
    >
      <SelectValue :placeholder="placeholder">
        <template v-if="current">
          <Icon
            v-if="current.icon"
            :name="current.icon"
            mode="svg"
            class="size-4 shrink-0"
          />
          <span class="truncate">{{ current.label }}</span>
        </template>
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
        <Icon
          v-if="option.icon"
          :name="option.icon"
          mode="svg"
          class="size-4 shrink-0"
        />
        {{ option.label }}
      </SelectItem>
    </SelectContent>
  </Select>
</template>
