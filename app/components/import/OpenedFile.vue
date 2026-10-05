<script setup lang="ts">
const castStore = useCastStore()
const announceImport = useImportToast()

const open = computed({
  get: () => !!castStore.current,
  set: (value: boolean) => {
    if (!value) castStore.dismissOpened()
  },
})

function onImported(instanceId: string, updated: boolean) {
  castStore.dismissOpened()
  announceImport(instanceId, updated)
}
</script>

<template>
  <UModal
    v-model:open="open"
    :title="$t('import_pack.opened_title')"
  >
    <template #body>
      <ImportPackModalBody
        v-if="castStore.current"
        :key="castStore.current"
        :path="castStore.current"
        @imported="onImported"
      />
    </template>
  </UModal>
</template>
