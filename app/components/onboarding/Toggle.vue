<script setup lang="ts">
defineProps<{
  label: string
  hint: string
  icon: string
}>()

const model = defineModel<boolean>({required: true})
</script>

<template>
  <div
      class="group relative border transition-colors duration-500"
      :class="model ? 'border-acid/45 bg-ink-800' : 'border-line bg-ink-800/50 hover:border-line-strong'"
  >
    <span
        class="absolute inset-y-0 left-0 w-[2px] origin-top bg-acid transition-transform duration-500 ease-deck"
        :class="model ? 'scale-y-100' : 'scale-y-0'"
    />

    <div class="flex cursor-pointer items-start gap-5 px-6 py-5" @click="model = !model">
      <UIcon
          :name="icon"
          class="mt-0.5 size-5 shrink-0 transition-all duration-500 ease-deck"
          :class="model ? 'text-acid scale-110' : 'text-fg-faint'"
      />

      <div class="min-w-0 flex-1">
        <p
            class="font-mono text-[10px] uppercase tracking-[0.24em] transition-colors duration-500"
            :class="model ? 'text-fg' : 'text-fg-faint'"
        >
          {{ label }}
        </p>
        <p class="mt-2 text-[12px] leading-relaxed text-fg-muted">{{ hint }}</p>
      </div>

      <USwitch v-model="model" size="lg" class="mt-0.5 shrink-0" @click.stop/>
    </div>

    <div v-if="$slots.default" class="border-t border-line px-6 py-5">
      <slot :on="model"/>
    </div>
  </div>
</template>
