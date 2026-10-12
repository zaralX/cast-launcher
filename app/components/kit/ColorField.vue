<script setup lang="ts">
const props = defineProps<{
  label: string
  alpha?: boolean
}>()

const model = defineModel<string>()

const draft = ref(model.value ?? '')

watch(model, (value) => {
  if (value !== parseThemeColor(draft.value)) draft.value = value ?? ''
})

const rgb = computed(() => (isThemeColor(model.value) ? model.value!.slice(0, 7) : '#000000'))

const opacity = computed({
  get: () => Math.round(alphaOf(model.value) * 100),
  set: (value: number | string) => {
    const percent = Math.min(100, Math.max(0, Number(value) || 0))
    model.value = withAlpha(rgb.value, percent / 100)
  },
})

function pick(event: Event) {
  model.value = withAlpha((event.target as HTMLInputElement).value, props.alpha ? alphaOf(model.value) : 1)
}

function commit() {
  const value = parseThemeColor(draft.value)
  if (value) model.value = value
  else draft.value = model.value ?? ''
}
</script>

<template>
  <div class="flex items-center gap-2.5 py-1">
    <span class="relative size-7 shrink-0 overflow-hidden rounded-sm bg-[repeating-conic-gradient(var(--cast-line-strong)_0_25%,transparent_0_50%)] ring-1 ring-line-strong [background-size:8px_8px]">
      <span
        class="absolute inset-0"
        :style="{ background: model }"
      />
      <input
        type="color"
        :value="rgb"
        :aria-label="label"
        class="absolute inset-0 size-full cursor-pointer opacity-0"
        @input="pick"
      >
    </span>

    <span class="min-w-0 flex-1 truncate text-body text-fg">{{ label }}</span>

    <Input
      v-model="draft"
      font="mono"
      maxlength="9"
      spellcheck="false"
      :aria-label="label"
      class="w-23"
      @change="commit"
      @keydown.enter="commit"
    />

    <KitNumberInput
      v-if="alpha"
      v-model="opacity"
      unit="%"
      min="0"
      max="100"
      :aria-label="$t('theme_editor.opacity', { label })"
      class="w-19"
    />
  </div>
</template>
