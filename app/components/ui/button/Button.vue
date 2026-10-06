<script setup lang="ts">
import type { PrimitiveProps } from 'reka-ui'
import type { HTMLAttributes } from 'vue'
import type { RouteLocationRaw } from 'vue-router'
import type { ButtonVariants } from '.'
import { Primitive } from 'reka-ui'
import { cn } from '@/lib/utils'
import { buttonVariants } from '.'

interface Props extends PrimitiveProps {
  variant?: ButtonVariants['variant']
  size?: ButtonVariants['size']
  class?: HTMLAttributes['class']
  icon?: string
  trailingIcon?: string
  loading?: boolean
  disabled?: boolean
  type?: 'button' | 'submit' | 'reset'
  to?: RouteLocationRaw
}

const props = withDefaults(defineProps<Props>(), {
  as: 'button',
  type: 'button',
})

const NuxtLink = resolveComponent('NuxtLink')

const isLink = computed(() => props.to != null)
const resolvedAs = computed(() => (isLink.value ? NuxtLink : props.as))
const isDisabled = computed(() => props.disabled || props.loading)
const iconClass = computed(() => (props.size === 'xs' ? 'size-3' : 'size-3.5'))
</script>

<template>
  <Primitive
    data-slot="button"
    :data-variant="variant"
    :data-size="size"
    :as="resolvedAs"
    :as-child="asChild"
    :to="to"
    :type="isLink || asChild ? undefined : type"
    :disabled="isLink ? undefined : isDisabled"
    :aria-disabled="isLink && isDisabled ? true : undefined"
    :class="cn(buttonVariants({ variant, size }), props.class)"
  >
    <Spinner
      v-if="loading"
      :class="iconClass"
    />
    <Icon
      v-else-if="icon"
      :name="icon"
      :class="cn('shrink-0', iconClass)"
    />
    <slot />
    <Icon
      v-if="trailingIcon"
      :name="trailingIcon"
      :class="cn('shrink-0', iconClass)"
    />
  </Primitive>
</template>
