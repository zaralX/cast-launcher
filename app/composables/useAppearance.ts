import { usePreferredDark } from '@vueuse/core'

export function useTheme() {
  const store = useAppStore()
  const prefersDark = usePreferredDark()

  return computed(() => {
    const theme = themeOf(store.config?.launcher.theme)
    if (theme !== SYSTEM_THEME) return theme
    return prefersDark.value ? DEFAULT_THEME : 'light'
  })
}

export function useAccent() {
  const store = useAppStore()
  return computed(() => accentOf(store.config?.launcher.accent))
}

export function useCompact() {
  const store = useAppStore()
  return computed(() => store.config?.launcher.compact === true)
}

export function useAppearance() {
  const theme = useTheme()
  const accent = useAccent()
  const compact = useCompact()

  watchEffect(() => {
    const root = document.documentElement
    root.dataset.theme = theme.value

    if (isCustomAccent(accent.value)) {
      delete root.dataset.accent
      root.style.setProperty('--cast-accent', accent.value)
      root.style.setProperty('--cast-on-accent', onAccentOf(accent.value))
      return
    }

    root.style.removeProperty('--cast-accent')
    root.style.removeProperty('--cast-on-accent')
    if (accent.value === DEFAULT_ACCENT) delete root.dataset.accent
    else root.dataset.accent = accent.value
  })

  return { theme, accent, compact }
}
