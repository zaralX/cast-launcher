<script setup lang="ts" generic="T extends string | number, M extends boolean = false">
import type { HTMLAttributes } from 'vue'
import type { SelectOption } from '~/types/ui'
import { FIELD } from '@/lib/styles'
import { cn } from '@/lib/utils'

const props = defineProps<{
  items: readonly (T | SelectOption<T>)[]
  multiple?: M
  placeholder?: string
  searchPlaceholder?: string
  disabled?: boolean
  class?: HTMLAttributes['class']
}>()

const model = defineModel<M extends true ? T[] : T>()

const options = computed<SelectOption<T>[]>(() => props.items.map(item =>
  typeof item === 'object' ? item : { label: String(item), value: item },
))

const selected = computed(() => {
  const value = model.value
  const values = (Array.isArray(value) ? value : value == null ? [] : [value]) as T[]
  return options.value.filter(option => values.includes(option.value))
})

const display = computed(() => selected.value.map(option => option.label).join(', '))
</script>

<template>
  <Combobox
    v-model="model"
    :multiple="multiple"
    :disabled="disabled"
  >
    <ComboboxAnchor>
      <ComboboxTrigger as-child>
        <button
          type="button"
          :disabled="disabled"
          :class="cn(FIELD, 'flex h-8 w-full cursor-pointer items-center justify-between gap-1.5 pr-2 pl-2.5 text-left', props.class)"
        >
          <span
            class="truncate"
            :class="!display && 'text-fg-faint'"
          >{{ display || placeholder }}</span>
          <Icon
            name="i-lucide-chevron-down"
            class="size-5 shrink-0 text-fg-faint"
          />
        </button>
      </ComboboxTrigger>
    </ComboboxAnchor>

    <ComboboxList>
      <ComboboxInput
        :placeholder="searchPlaceholder ?? $t('common.search')"
        :display-value="() => ''"
        auto-focus
      />
      <ComboboxEmpty>{{ $t('common.no_match') }}</ComboboxEmpty>
      <ComboboxViewport>
        <ComboboxItem
          v-for="option in options"
          :key="option.value"
          :value="option.value"
        >
          <span class="truncate">{{ option.label }}</span>
          <ComboboxItemIndicator>
            <Icon
              name="i-lucide-check"
              class="size-4 text-acid"
            />
          </ComboboxItemIndicator>
        </ComboboxItem>
      </ComboboxViewport>
    </ComboboxList>
  </Combobox>
</template>
