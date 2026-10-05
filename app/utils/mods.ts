import type { CatalogMatch, ModFile } from '~/types/mods'
import { MOD_EXTENSIONS } from '~/types/mods'

export function isModFile(name: string): boolean {
  return MOD_EXTENSIONS.includes(name.split('.').pop()?.toLowerCase() ?? '')
}

export function modKey(mod: ModFile): string {
  return mod.path.replace(/\.disabled$/, '')
}

export function modName(mod: ModFile, matched?: CatalogMatch | null): string {
  return mod.details.name.trim() || matched?.title.trim() || mod.fileName
}

export function modAuthors(mod: ModFile): string {
  return mod.details.authors.join(', ')
}

export function modSize(bytes: number): string {
  const { $i18n } = useNuxtApp()

  if (bytes <= 0) return ''
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} ${$i18n.t('common.unit.kb')}`

  return `${(bytes / 1024 / 1024).toFixed(1)} ${$i18n.t('common.unit.mb')}`
}

export function matchesMod(mod: ModFile, query: string, matched?: CatalogMatch | null): boolean {
  const needle = query.trim().toLowerCase()
  if (!needle) return true

  return [modName(mod, matched), mod.details.modId, mod.fileName, modAuthors(mod), matched?.title ?? '']
    .some(field => field.toLowerCase().includes(needle))
}
