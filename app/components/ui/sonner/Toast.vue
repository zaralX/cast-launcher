<script setup lang="ts">
import type { ToastAction, ToastColor } from '.'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

const props = withDefaults(defineProps<{
  title: string
  description?: string
  icon?: string
  color?: ToastColor
  duration: number
  actions?: ToastAction[]
  isPaused?: boolean
}>(), {
  description: undefined,
  icon: undefined,
  color: 'neutral',
  actions: () => [],
  isPaused: false,
})

const emit = defineEmits<{ closeToast: [] }>()

const TONE: Record<ToastColor, { icon: string, bar: string }> = {
  neutral: { icon: 'text-fg-muted', bar: 'bg-fg-faint' },
  info: { icon: 'text-accent', bar: 'bg-accent' },
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
    class="relative flex w-(--width) gap-3 overflow-hidden rounded-md bg-raise pt-3.5 pr-3 pb-4 pl-3.5 shadow-float ring-1 ring-line-strong"
  >
    <Icon
      v-if="icon"
      :name="icon"
      :class="cn('size-5 shrink-0', TONE[color].icon)"
    />

    <div class="flex w-0 flex-1 flex-col">
      <p class="text-title font-medium text-fg">
        {{ title }}
      </p>
      <p
        v-if="description"
        class="mt-0.5 text-body text-fg-muted"
      >
        {{ description }}
      </p>
      <div
        v-if="actions.length"
        class="mt-2.5 flex items-start gap-1.5"
      >
        <Button
          v-for="(action, index) in actions"
          :key="action.label"
          :variant="index === 0 ? 'secondary' : 'ghost'"
          size="sm"
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
      class="-mt-1 shrink-0"
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
