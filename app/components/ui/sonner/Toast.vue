<script setup lang="ts">
import type { ToastAction, ToastColor } from '.'
import { cn } from '@/lib/utils'

const props = withDefaults(defineProps<{
  title: string
  description?: string
  icon?: string
  color?: ToastColor
  duration?: number
  actions?: ToastAction[]
  isPaused?: boolean
}>(), {
  description: undefined,
  icon: undefined,
  color: 'neutral',
  duration: 5000,
  actions: () => [],
  isPaused: false,
})

const emit = defineEmits<{ closeToast: [] }>()

const TONE: Record<ToastColor, { icon: string, bar: string }> = {
  neutral: { icon: 'text-fg', bar: 'bg-fg-muted' },
  info: { icon: 'text-acid', bar: 'bg-acid' },
  success: { icon: 'text-success', bar: 'bg-success' },
  warning: { icon: 'text-warning', bar: 'bg-warning' },
  error: { icon: 'text-danger', bar: 'bg-danger' },
}

function run(action: ToastAction, event: MouseEvent) {
  action.onClick(event)
  emit('closeToast')
}
</script>

<template>
  <div
    data-slot="toast"
    class="relative flex w-(--width) gap-2.5 overflow-hidden bg-ink-800 p-4 shadow-lg ring ring-line"
  >
    <Icon
      v-if="icon"
      :name="icon"
      :class="cn('size-5 shrink-0', TONE[color].icon)"
    />

    <div class="flex w-0 flex-1 flex-col">
      <p class="text-lead font-medium text-fg">
        {{ title }}
      </p>
      <p
        v-if="description"
        class="mt-1 text-lead text-fg-muted"
      >
        {{ description }}
      </p>
      <div
        v-if="actions.length"
        class="mt-2.5 flex items-start gap-1.5"
      >
        <Button
          v-for="action in actions"
          :key="action.label"
          variant="outline"
          size="xs"
          @click="run(action, $event)"
        >
          {{ action.label }}
        </Button>
      </div>
    </div>

    <Button
      variant="ghost"
      size="icon-sm"
      icon="i-lucide-x"
      :aria-label="$t('common.close')"
      class="-mt-1.5 -mr-1.5 shrink-0"
      @click="emit('closeToast')"
    />

    <span
      v-if="Number.isFinite(props.duration)"
      :class="cn('absolute inset-x-0 bottom-0 h-0.5 origin-left', TONE[color].bar)"
      :style="{
        animation: `toast-timer ${props.duration}ms linear forwards`,
        animationPlayState: isPaused ? 'paused' : 'running',
      }"
    />
  </div>
</template>
