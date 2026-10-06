export function useAccent() {
  const store = useAppStore()
  return computed(() => accentOf(store.config?.launcher.accent))
}

export function useCompact() {
  const store = useAppStore()
  return computed(() => store.config?.launcher.compact === true)
}

export function useAppearance() {
  const accent = useAccent()
  const compact = useCompact()

  watchEffect(() => {
    document.documentElement.dataset.accent = accent.value
  })

  watchEffect(() => {
    document.documentElement.classList.toggle('compact', compact.value)
  })

  return { accent, compact }
}
