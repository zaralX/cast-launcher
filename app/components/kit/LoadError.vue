<script setup lang="ts">
import type { LauncherError } from '~/utils/error'

defineProps<{
  error: LauncherError
}>()

const emit = defineEmits<{ retry: [] }>()
</script>

<template>
  <Alert
    variant="danger"
    :icon="error.icon"
  >
    <AlertTitle>{{ error.title }}</AlertTitle>
    <AlertDescription v-if="error.reason ?? error.hint">
      {{ error.reason ?? error.hint }}
    </AlertDescription>
    <template #action>
      <Button
        size="sm"
        icon="i-lucide-rotate-cw"
        @click="emit('retry')"
      >
        {{ $t('common.retry') }}
      </Button>
    </template>
  </Alert>
</template>
