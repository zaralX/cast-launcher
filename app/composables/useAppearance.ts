import { usePreferredDark } from '@vueuse/core'

export function useCustomThemes() {
  const store = useAppStore()
  return computed(() => store.config?.launcher.themes ?? [])
}

export function useTheme() {
  const store = useAppStore()
  const custom = useCustomThemes()
  const prefersDark = usePreferredDark()

  return computed(() => {
    const launcher = store.config?.launcher
    const theme = themeOf(launcher?.theme, custom.value)
    if (theme !== SYSTEM_THEME) return theme

    return prefersDark.value
      ? pairThemeOf(launcher?.theme_dark, custom.value, DEFAULT_THEME)
      : pairThemeOf(launcher?.theme_light, custom.value, DEFAULT_LIGHT_THEME)
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
  const custom = useCustomThemes()

  watchEffect(() => {
    const root = document.documentElement
    const own = custom.value.find(item => item.id === theme.value)

    root.dataset.theme = own ? customThemeBase(own) : theme.value

    if (own ? own.classic : theme.value === 'classic') root.dataset.classic = ''
    else delete root.dataset.classic

    for (const token of THEME_TOKENS) root.style.removeProperty(`--cast-${token}`)

    for (const [token, value] of Object.entries(own?.colors ?? {})) {
      if (token === 'accent' && accent.value !== DEFAULT_ACCENT) continue
      if (isThemeColor(value)) root.style.setProperty(`--cast-${token}`, value)
    }

    if (isCustomAccent(accent.value)) {
      delete root.dataset.accent
      root.style.setProperty('--cast-accent', accent.value)
      root.style.setProperty('--cast-on-accent', onAccentOf(accent.value))
      return
    }

    if (accent.value === DEFAULT_ACCENT) delete root.dataset.accent
    else root.dataset.accent = accent.value
  })

  return { theme, accent, compact }
}
