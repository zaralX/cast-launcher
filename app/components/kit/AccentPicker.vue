<script setup lang="ts">
const model = defineModel<string>()

const preset = computed({
  get: () => {
    const accent = accentOf(model.value)
    return isCustomAccent(accent) ? undefined : accent
  },
  set: (value: string | undefined) => {
    if (value) model.value = value
  },
})

const hex = ref(isCustomAccent(model.value) ? model.value! : '')

watch(model, (value) => {
  if (isCustomAccent(value)) hex.value = value!
  else if (value) hex.value = ''
})

const normalized = computed(() => {
  const value = hex.value.trim()
  return value.startsWith('#') ? value : `#${value}`
})

const invalid = computed(() => hex.value.trim() !== '' && !isCustomAccent(normalized.value))

watch(normalized, (value) => {
  if (isCustomAccent(value)) model.value = value.toLowerCase()
})

// "From the theme" is drawn with the accents of the themes it can take.
const themeSwatch = computed(() => {
  const colors = [...new Set(THEMES.map(theme => theme.preview.accent))]
  const step = 100 / colors.length
  return `conic-gradient(${colors.map((color, i) => `${color} ${i * step}% ${(i + 1) * step}%`).join(', ')})`
})

const SWATCH = 'size-7 rounded-pill transition-shadow hover:ring-2 hover:ring-line-strong data-[state=checked]:ring-2 data-[state=checked]:ring-fg data-[state=checked]:ring-offset-2 data-[state=checked]:ring-offset-sheet'
</script>

<template>
  <div class="flex flex-wrap items-center gap-3.5">
    <RadioGroup
      v-model="preset"
      variant="bare"
    >
      <RadioGroupItem
        :value="DEFAULT_ACCENT"
        :title="$t('accent.theme')"
        :aria-label="$t('accent.theme')"
        :class="SWATCH"
        :style="{ background: themeSwatch }"
      />
      <RadioGroupItem
        v-for="accent in ACCENTS"
        :key="accent.value"
        :value="accent.value"
        :title="$t(accent.labelKey)"
        :aria-label="$t(accent.labelKey)"
        :class="SWATCH"
        :style="{ background: accent.preview }"
      />
    </RadioGroup>

    <Separator
      orientation="vertical"
      class="h-6 bg-line-strong data-[orientation=vertical]:h-6"
    />

    <InputGroup class="w-36">
      <InputGroupAddon>
        <span class="size-3.5 rounded-xs bg-accent" />
      </InputGroupAddon>
      <InputGroupInput
        v-model="hex"
        font="mono"
        placeholder="#HEX"
        maxlength="7"
        :aria-label="$t('accent.custom')"
        :aria-invalid="invalid || undefined"
      />
    </InputGroup>
  </div>
</template>
