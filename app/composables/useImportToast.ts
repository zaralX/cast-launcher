export function useImportToast() {
  const { t } = useI18n()
  const toast = useAppToast()
  const instanceStore = useInstanceStore()

  return (instanceId: string, updated: boolean) => {
    const name = instanceStore.getInstance(instanceId)?.name ?? t('home.imported_fallback')

    toast.add({
      title: t(updated ? 'home.updated_from_file' : 'home.imported', { name }),
      color: 'success',
      icon: 'i-lucide-file-archive',
    })
  }
}
