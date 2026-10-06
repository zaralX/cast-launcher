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
    class="gap-3 p-5"
  >
    <AlertTitle>{{ error.title }}</AlertTitle>
    <AlertDescription v-if="error.reason ?? error.hint">
      {{ error.reason ?? error.hint }}
    </AlertDescription>
    <Button
      variant="quiet"
      icon="i-lucide-rotate-cw"
      class="mt-2"
      @click="emit('retry')"
    >
      {{ $t('common.retry') }}
    </Button>
  </Alert>
</template>
