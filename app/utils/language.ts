const LOCALE_FLAGS: Record<string, string | undefined> = {
  ru: 'flag:ru-1x1',
  en: 'flag:gb-1x1',
}

export function flagOf(code: string): string | null {
  return LOCALE_FLAGS[code] ?? null
}
