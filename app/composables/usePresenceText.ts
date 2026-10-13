export function usePresenceText() {
  const { t, locale } = useI18n()

  watch(locale, () => safeRun(() => call('set_presence_text', {
    text: {
      menu: t('presence.menu'),
      playing: t('presence.playing'),
      button: t('presence.button'),
    },
  })), { immediate: true })
}
