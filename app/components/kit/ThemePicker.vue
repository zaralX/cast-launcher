<script setup lang="ts">
import type { CustomTheme, ThemePreview } from '~/types/app'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  custom?: CustomTheme[]
  light?: string
  dark?: string
}>(), {
  custom: () => [],
  light: DEFAULT_LIGHT_THEME,
  dark: DEFAULT_THEME,
})

const model = defineModel<string>()

const current = computed({
  get: () => themeOf(model.value, props.custom),
  set: (value: string) => {
    model.value = value
  },
})

const pair = computed(() => [
  themePreview(pairThemeOf(props.light, props.custom, DEFAULT_LIGHT_THEME), props.custom),
  themePreview(pairThemeOf(props.dark, props.custom, DEFAULT_THEME), props.custom),
])

const options = computed(() => [
  ...THEMES.map(theme => ({ value: theme.value, preview: theme.preview })),
  ...props.custom.map(theme => ({ value: theme.id, preview: themePreview(theme.id, props.custom) })),
])

const TILE = 'relative block h-16.5 overflow-hidden rounded-md ring-1 ring-line-strong transition-shadow group-hover:ring-fg-faint group-data-[state=checked]:ring-2 group-data-[state=checked]:ring-accent'

function sheetStyle(preview: ThemePreview) {
  return {
    background: preview.sheet,
    boxShadow: preview.sheetEdge ? `inset 1px 1px 0 ${preview.sheetEdge}` : undefined,
  }
}

function cardStyle(preview: ThemePreview) {
  return {
    background: preview.panel,
    boxShadow: preview.cardLine ? `inset 0 0 0 1px ${preview.cardLine}` : undefined,
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
      <span :class="TILE">
        <span
          v-for="(preview, index) in pair"
          :key="index"
          class="absolute inset-y-0 w-1/2 overflow-hidden"
          :class="index === 0 ? 'left-0' : 'right-0'"
          :style="{ background: preview.frame }"
        >
          <span
            :class="cn('absolute top-2 right-0 bottom-0 grid content-start gap-1 rounded-tl-sm p-1.5', index === 0 ? 'left-3' : 'left-0', preview.square && 'rounded-none')"
            :style="sheetStyle(preview)"
          >
            <span
              :class="cn('block h-4 rounded-xs', preview.square && 'rounded-none')"
              :style="cardStyle(preview)"
            />
            <span
              :class="cn('block h-1.5 w-3/5 rounded-xs', preview.square && 'rounded-none')"
              :style="{ background: preview.accent }"
            />
          </span>
        </span>
      </span>
      <span class="flex items-center gap-1.5 text-caption font-medium text-fg">
        {{ $t('theme.system') }}
        <Icon
          v-if="current === SYSTEM_THEME"
          name="i-lucide-check"
          class="size-3.5 text-accent"
        />
      </span>
    </RadioGroupItem>

    <RadioGroupItem
      v-for="option in options"
      :key="option.value"
      :value="option.value"
      class="w-26 flex-col items-stretch gap-1.5 text-left"
    >
      <KitThemeSample
        :preview="option.preview"
        :class="TILE"
      />
      <span class="flex min-w-0 items-center gap-1.5 text-caption font-medium text-fg">
        <span class="truncate">{{ themeName(option.value, custom) }}</span>
        <Icon
          v-if="current === option.value"
          name="i-lucide-check"
          class="size-3.5 shrink-0 text-accent"
        />
      </span>
    </RadioGroupItem>
  </RadioGroup>
</template>
