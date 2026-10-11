import type { Accent, Theme } from '~/types/app'

export const DEFAULT_THEME = 'depth'

export const SYSTEM_THEME = 'system'

export const DEFAULT_ACCENT = 'theme'

// The preview colors mirror main.css, a theme card draws them before the theme is applied.
export const THEMES: Theme[] = [
  {
    value: 'depth',
    labelKey: 'theme.depth',
    preview: {
      frame: '#070a0f',
      sheet: '#0f141c',
      panel: '#1b2330',
      accent: '#2fb4f4',
    },
  },
  {
    value: 'coal',
    labelKey: 'theme.coal',
    preview: {
      frame: '#070707',
      sheet: '#111111',
      panel: '#1d1d1d',
      accent: '#2fb4f4',
    },
  },
  {
    value: 'oled',
    labelKey: 'theme.oled',
    preview: {
      frame: '#000000',
      sheet: '#000000',
      panel: '#10141b',
      accent: '#2fb4f4',
      edge: 'rgb(150 180 220 / 0.18)',
    },
  },
  {
    value: 'light',
    labelKey: 'theme.light',
    preview: {
      frame: '#dde2e9',
      sheet: '#f3f5f8',
      panel: '#ffffff',
      accent: '#0a6fa8',
      edge: 'rgb(15 35 60 / 0.1)',
    },
  },
  {
    value: 'classic',
    labelKey: 'theme.classic',
    square: true,
    preview: {
      frame: '#0b0d10',
      sheet: '#07080a',
      panel: '#0b0d10',
      accent: '#38bdf8',
      edge: 'rgb(255 255 255 / 0.16)',
    },
  },
  {
    value: 'nether',
    labelKey: 'theme.nether',
    preview: {
      frame: '#0d0606',
      sheet: '#160b0a',
      panel: '#2b1713',
      accent: '#ff8a3d',
    },
  },
  {
    value: 'end',
    labelKey: 'theme.end',
    preview: {
      frame: '#09080e',
      sheet: '#100e17',
      panel: '#201c2c',
      accent: '#dfe58a',
    },
  },
  {
    value: 'moss',
    labelKey: 'theme.moss',
    preview: {
      frame: '#060b07',
      sheet: '#0c140e',
      panel: '#162218',
      accent: '#f3a641',
    },
  },
]

export const ACCENTS: Accent[] = [
  { value: 'sky', labelKey: 'accent.sky', preview: '#2fb4f4' },
  { value: 'grass', labelKey: 'accent.grass', preview: '#46d483' },
  { value: 'amber', labelKey: 'accent.amber', preview: '#f5b83d' },
  { value: 'coral', labelKey: 'accent.coral', preview: '#ff7f6e' },
  { value: 'violet', labelKey: 'accent.violet', preview: '#a78bfa' },
]

const HEX = /^#[0-9a-f]{6}$/i

export function themeOf(value: string | undefined | null): string {
  if (value === SYSTEM_THEME) return SYSTEM_THEME
  return THEMES.some(theme => theme.value === value) ? value! : DEFAULT_THEME
}

export function isCustomAccent(value: string | undefined | null): boolean {
  return value != null && HEX.test(value)
}

export function accentOf(value: string | undefined | null): string {
  if (isCustomAccent(value)) return value!.toLowerCase()
  return ACCENTS.some(accent => accent.value === value) ? value! : DEFAULT_ACCENT
}

// Text on a custom accent: near-black on light colors, white on dark ones (WCAG relative luminance).
export function onAccentOf(hex: string): string {
  const channel = (offset: number) => {
    const value = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
  }
  const luminance = 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
  return luminance > 0.18 ? '#0b0d10' : '#ffffff'
}
