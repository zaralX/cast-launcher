<script setup lang="ts">
import type { Theme } from '~/types/app'
import { cn } from '@/lib/utils'

const model = defineModel<string>()

const current = computed({
  get: () => themeOf(model.value),
  set: (value: string) => {
    model.value = value
  },
})

const light = THEMES.find(theme => theme.value === 'light')!
const depth = THEMES.find(theme => theme.value === DEFAULT_THEME)!

function sheetStyle(theme: Theme) {
  return {
    background: theme.preview.sheet,
    boxShadow: theme.preview.edge && theme.value !== 'light' ? `inset 1px 1px 0 ${theme.preview.edge}` : undefined,
  }
}

function tileStyle(theme: Theme) {
  return {
    background: theme.preview.panel,
    boxShadow: theme.value === 'light' || theme.square ? `inset 0 0 0 1px ${theme.preview.edge}` : undefined,
  }
}
</script>

<template>
  <RadioGroup
    v-model="current"
    variant="bare"
    class="gap-3.5"
  >
    <RadioGroupItem
      :value="SYSTEM_THEME"
      class="w-26 flex-col items-stretch gap-1.5 text-left"
    >
      <span class="relative block h-16.5 overflow-hidden rounded-md ring-1 ring-line-strong transition-shadow group-hover:ring-fg-faint group-data-[state=checked]:ring-2 group-data-[state=checked]:ring-accent">
        <span
          v-for="(theme, index) in [light, depth]"
          :key="theme.value"
          class="absolute inset-y-0 w-1/2 overflow-hidden"
          :class="index === 0 ? 'left-0' : 'right-0'"
          :style="{ background: theme.preview.frame }"
        >
          <span
            class="absolute top-2 right-0 bottom-0 grid content-start gap-1 rounded-tl-sm p-1.5"
            :class="index === 0 ? 'left-3' : 'left-0'"
            :style="sheetStyle(theme)"
          >
            <span
              class="block h-4 rounded-xs"
              :style="tileStyle(theme)"
            />
            <span
              class="block h-1.5 w-3/5 rounded-xs"
              :style="{ background: theme.preview.accent }"
            />
          </span>
        </span>
      </span>
      <span class="text-caption font-medium text-fg">
        {{ $t('theme.system') }}
        <span class="block text-micro font-normal text-fg-faint">{{ $t('theme.system_hint') }}</span>
      </span>
    </RadioGroupItem>

    <RadioGroupItem
      v-for="theme in THEMES"
      :key="theme.value"
      :value="theme.value"
      class="w-26 flex-col items-stretch gap-1.5 text-left"
    >
      <span
        class="relative block h-16.5 overflow-hidden rounded-md ring-1 ring-line-strong transition-shadow group-hover:ring-fg-faint group-data-[state=checked]:ring-2 group-data-[state=checked]:ring-accent"
        :style="{ background: theme.preview.frame }"
      >
        <span
          :class="cn('absolute top-2 right-0 bottom-0 left-3 grid grid-cols-2 content-start gap-1 rounded-tl-sm p-1.5', theme.square && 'rounded-none')"
          :style="sheetStyle(theme)"
        >
          <span
            v-for="tile in 2"
            :key="tile"
            :class="cn('block h-4 rounded-xs', theme.square && 'rounded-none')"
            :style="tileStyle(theme)"
          />
          <span
            :class="cn('block h-1.5 w-3/5 rounded-xs', theme.square && 'rounded-none')"
            :style="{ background: theme.preview.accent }"
          />
        </span>
      </span>
      <span class="flex items-center gap-1.5 text-caption font-medium text-fg">
        {{ $t(theme.labelKey) }}
        <Icon
          v-if="current === theme.value"
          name="i-lucide-check"
          class="size-3.5 text-accent"
        />
      </span>
    </RadioGroupItem>
  </RadioGroup>
</template>
