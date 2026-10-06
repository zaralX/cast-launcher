export function useLanguage() {
  const { locale, localeCodes, defaultLocale, setLocale } = useI18n()
  const store = useAppStore()

  watch(() => store.config?.launcher.language, (language) => {
    const next = localeCodes.value.find(code => code === language) ?? defaultLocale

    if (next !== locale.value) setLocale(next)
  }, { immediate: true })

  return locale
}

export function useAvailableLocales() {
  const { locales } = useI18n()

  return computed(() => locales.value.map(locale => ({ code: locale.code, name: locale.name ?? locale.code })))
}
