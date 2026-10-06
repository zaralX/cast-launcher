<script setup lang="ts">
const model = defineModel<string>()
</script>

<template>
  <RadioGroup
    v-model="model"
    variant="tiles"
  >
    <RadioGroupItem
      v-for="accent in ACCENTS"
      :key="accent.value"
      :value="accent.value"
      :title="$t(accent.labelKey)"
      :aria-label="$t(accent.labelKey)"
      class="size-9"
    >
      <span
        class="size-4 transition-transform duration-300 ease-deck group-hover:scale-110"
        :style="{ backgroundColor: accent.preview }"
      />
    </RadioGroupItem>
  </RadioGroup>
</template>
