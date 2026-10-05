const UNITS = ['common.unit.b', 'common.unit.kb', 'common.unit.mb', 'common.unit.gb', 'common.unit.tb']

export function formatBytes(bytes: number): string {
  const { $i18n } = useNuxtApp()

  if (bytes <= 0) return `0 ${$i18n.t('common.unit.b')}`

  let value = bytes
  let unit = 0

  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024
    unit++
  }

  return `${value >= 100 || unit === 0 ? Math.round(value) : value.toFixed(1)} ${$i18n.t(UNITS[unit]!)}`
}
