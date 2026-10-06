<script setup lang="ts">
defineProps<{
  disabled?: boolean
}>()

const min = defineModel<number>('min', { required: true })
const max = defineModel<number>('max', { required: true })

const gb = (mb?: number) => ((mb ?? 0) / 1024).toFixed(1).replace('.', ',')

const minInput = computed({
  get: () => min.value,
  set: (value: string | number) => {
    min.value = Number(value) || 0
  },
})

const maxInput = computed({
  get: () => max.value,
  set: (value: string | number) => {
    max.value = Number(value) || 0
  },
})
</script>

<template>
  <div class="grid grid-cols-2 gap-6">
    <KitField
      :label="$t('settings.java.min_ram')"
      :hint="$t('settings.java.ram_hint', { value: gb(min) })"
    >
      <KitNumberInput
        v-model="minInput"
        :min="1"
        :disabled="disabled"
        :unit="$t('common.unit.mb')"
      />
    </KitField>

    <KitField
      :label="$t('settings.java.max_ram')"
      :hint="$t('settings.java.ram_hint', { value: gb(max) })"
    >
      <KitNumberInput
        v-model="maxInput"
        :min="min || 1"
        :disabled="disabled"
        :unit="$t('common.unit.mb')"
      />
    </KitField>
  </div>
</template>
