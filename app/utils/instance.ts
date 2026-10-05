import type { InstallStage, InstanceSettings, Playtime } from '~/types/instance'

export function emptyInstanceSettings(): InstanceSettings {
  return {
    overrideMemory: false,
    minRam: 0,
    maxRam: 0,
    overrideJava: false,
    javaMode: 'auto',
    javaPath: '',
  }
}

export function emptyPlaytime(): Playtime {
  return { totalSeconds: 0, lastSeconds: 0, lastPlayedAt: 0 }
}

export function formatPlaytime(seconds: number): string {
  const { $i18n } = useNuxtApp()

  if (!seconds || seconds < 0) return ''
  if (seconds < 60) return $i18n.t('common.playtime.less_minute')

  const days = Math.floor(seconds / 86400)
  const hours = Math.floor(seconds / 3600) % 24
  const minutes = Math.floor(seconds / 60) % 60

  const parts: string[] = []

  if (days) parts.push(`${days} ${$i18n.t('common.playtime.days')}`)
  if (hours) parts.push(`${hours} ${$i18n.t('common.playtime.hours')}`)
  if (minutes && !days) parts.push(`${minutes} ${$i18n.t('common.playtime.minutes')}`)

  return parts.join(' ')
}

export function formatLastPlayed(millis: number): string {
  if (!millis) return ''

  return new Date(millis).toLocaleString(useNuxtApp().$i18n.locale.value, {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

const TERMINAL_STAGES: InstallStage[] = ['finished', 'aborted', 'failed']

export function isTerminalStage(stage: InstallStage): boolean {
  return TERMINAL_STAGES.includes(stage)
}
