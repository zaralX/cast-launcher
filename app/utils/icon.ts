export function itemCategoryLabel(key: string): string {
  const { $i18n } = useNuxtApp()
  const messageKey = `item.category.${key}`

  return $i18n.te(messageKey) ? $i18n.t(messageKey) : key.replace(/_/g, ' ')
}

export function itemFallbackName(item: string): string {
  const words = item.replace(/_/g, ' ')
  return words.charAt(0).toUpperCase() + words.slice(1)
}
