import type { PackCategory, PackVersion } from '~/types/catalog'

const CATEGORY_LABELS: Record<string, string> = {
  fabric: 'Fabric',
  forge: 'Forge',
  neoforge: 'NeoForge',
  quilt: 'Quilt',
}

export function categoryLabel(category: PackCategory): string {
  return categoryName(category.id) || category.label.replace(/[-_]/g, ' ')
}

export function categoryName(name: string): string {
  const { $i18n } = useNuxtApp()
  const key = `catalog.category.${name}`

  if ($i18n.te(key)) return $i18n.t(key)

  return CATEGORY_LABELS[name] ?? name.replace(/[-_]/g, ' ')
}

export function formatDownloads(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return String(value)
}

export function versionLabel(version: PackVersion): string {
  const parts = [version.versionNumber || version.name]

  if (version.minecraftVersion) parts.push(version.minecraftVersion)
  if (version.loaders.length) parts.push(version.loaders.map(capitalize).join('/'))

  return parts.join(' · ')
}

export function unsupportedReason(version: PackVersion): string | null {
  const { $i18n } = useNuxtApp()

  if (version.supported) return null
  if (version.blocked) return $i18n.t('catalog.unsupported.blocked')

  if (!version.loader) {
    const loaders = version.loaders.length
      ? version.loaders.join(', ')
      : $i18n.t('catalog.unsupported.loader_unknown')

    return $i18n.t('catalog.unsupported.loader', { loaders })
  }

  if (!version.minecraftVersion) return $i18n.t('catalog.unsupported.minecraft')

  return $i18n.t('catalog.unsupported.archive')
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1)
}
