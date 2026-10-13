<script setup lang="ts">
import type { ButtonVariants } from '@/components/ui/button'

withDefaults(defineProps<{
  icon?: string
  variant?: ButtonVariants['variant']
  menuLabel: string
  disabled?: boolean
}>(), {
  icon: undefined,
  variant: 'secondary',
  disabled: false,
})

const emit = defineEmits<{ click: [event: MouseEvent] }>()
</script>

<template>
  <div class="inline-flex gap-0.5">
    <Button
      :variant="variant"
      :icon="icon"
      :disabled="disabled"
      class="rounded-r-none"
      @click="emit('click', $event)"
    >
      <slot />
    </Button>
    <DropdownMenu>
      <DropdownMenuTrigger as-child>
        <Button
          :variant="variant"
          size="icon"
          icon="i-lucide-chevron-down"
          :disabled="disabled"
          :aria-label="menuLabel"
          class="rounded-l-none"
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <slot name="menu" />
      </DropdownMenuContent>
    </DropdownMenu>
  </div>
</template>
