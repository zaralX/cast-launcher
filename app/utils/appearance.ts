import { v4 } from 'uuid'
import type { Accent, CustomTheme, Theme, ThemePreview, ThemeToken, ThemeTokenGroup } from '~/types/app'

export const DEFAULT_THEME = 'depth'

export const DEFAULT_LIGHT_THEME = 'light'

export const SYSTEM_THEME = 'system'

export const DEFAULT_ACCENT = 'theme'

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
      sheetEdge: 'rgb(150 180 220 / 0.18)',
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
      cardLine: 'rgb(15 35 60 / 0.1)',
    },
  },
  {
    value: 'classic',
    labelKey: 'theme.classic',
    preview: {
      frame: '#0b0d10',
      sheet: '#07080a',
      panel: '#0b0d10',
      accent: '#38bdf8',
      sheetEdge: 'rgb(255 255 255 / 0.16)',
      cardLine: 'rgb(255 255 255 / 0.16)',
      square: true,
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

export const THEME_TOKENS: ThemeToken[] = [
  'frame',
  'sheet',
  'panel',
  'control',
  'hover',
  'raise',
  'line',
  'line-strong',
  'card-line',
  'sheet-edge',
  'fg',
  'fg-muted',
  'fg-faint',
  'accent',
  'on-accent',
  'success',
  'warning',
  'danger',
  'on-danger',
]

export const THEME_TOKEN_GROUPS: ThemeTokenGroup[] = [
  {
    labelKey: 'theme_editor.group.surfaces',
    tokens: [
      { token: 'frame', labelKey: 'theme_editor.token.frame' },
      { token: 'sheet', labelKey: 'theme_editor.token.sheet' },
      { token: 'panel', labelKey: 'theme_editor.token.panel' },
      { token: 'control', labelKey: 'theme_editor.token.control' },
      { token: 'hover', labelKey: 'theme_editor.token.hover' },
      { token: 'raise', labelKey: 'theme_editor.token.raise' },
    ],
  },
  {
    labelKey: 'theme_editor.group.text',
    tokens: [
      { token: 'fg', labelKey: 'theme_editor.token.fg' },
      { token: 'fg-muted', labelKey: 'theme_editor.token.fg_muted' },
      { token: 'fg-faint', labelKey: 'theme_editor.token.fg_faint' },
    ],
  },
  {
    labelKey: 'theme_editor.group.lines',
    tokens: [
      { token: 'line', labelKey: 'theme_editor.token.line', alpha: true },
      { token: 'line-strong', labelKey: 'theme_editor.token.line_strong', alpha: true },
      { token: 'card-line', labelKey: 'theme_editor.token.card_line', alpha: true },
      { token: 'sheet-edge', labelKey: 'theme_editor.token.sheet_edge', alpha: true },
    ],
  },
  {
    labelKey: 'theme_editor.group.accent',
    tokens: [
      { token: 'accent', labelKey: 'theme_editor.token.accent' },
      { token: 'on-accent', labelKey: 'theme_editor.token.on_accent' },
    ],
  },
  {
    labelKey: 'theme_editor.group.states',
    tokens: [
      { token: 'success', labelKey: 'theme_editor.token.success' },
      { token: 'warning', labelKey: 'theme_editor.token.warning' },
      { token: 'danger', labelKey: 'theme_editor.token.danger' },
      { token: 'on-danger', labelKey: 'theme_editor.token.on_danger' },
    ],
  },
]

const HEX = /^#[0-9a-f]{6}$/i

const THEME_COLOR = /^#[0-9a-f]{6}(?:[0-9a-f]{2})?$/i

export function isBuiltInTheme(value: string | undefined | null): boolean {
  return THEMES.some(theme => theme.value === value)
}

export function themeOf(value: string | undefined | null, custom: CustomTheme[] = []): string {
  if (value === SYSTEM_THEME) return SYSTEM_THEME
  return pairThemeOf(value, custom, DEFAULT_THEME)
}

export function pairThemeOf(value: string | undefined | null, custom: CustomTheme[], fallback: string): string {
  if (isBuiltInTheme(value) || custom.some(theme => theme.id === value)) return value!
  return fallback
}

export function newThemeId(): string {
  return `custom-${v4().replaceAll('-', '')}`
}

export function themeName(value: string, custom: CustomTheme[] = []): string {
  const { $i18n } = useNuxtApp()

  const own = custom.find(theme => theme.id === value)
  if (own) return own.name.trim() || $i18n.t('theme.custom.untitled')

  const builtIn = THEMES.find(theme => theme.value === value)
  return builtIn ? $i18n.t(builtIn.labelKey) : value
}

export function customThemeBase(theme: CustomTheme): string {
  const sheet = theme.colors.sheet
  return isThemeColor(sheet) && luminanceOf(sheet!) > 0.4 ? DEFAULT_LIGHT_THEME : DEFAULT_THEME
}

export function themePreview(value: string, custom: CustomTheme[] = []): ThemePreview {
  const own = custom.find(theme => theme.id === value)
  const base = THEMES.find(theme => theme.value === (own ? customThemeBase(own) : value)) ?? THEMES[0]!

  if (!own) return base.preview

  return {
    frame: own.colors.frame ?? base.preview.frame,
    sheet: own.colors.sheet ?? base.preview.sheet,
    panel: own.colors.panel ?? base.preview.panel,
    accent: own.colors.accent ?? base.preview.accent,
    sheetEdge: own.colors['sheet-edge'] ?? base.preview.sheetEdge,
    cardLine: own.colors['card-line'] ?? base.preview.cardLine,
    square: own.classic,
  }
}

export function isThemeColor(value: string | undefined | null): boolean {
  return value != null && THEME_COLOR.test(value)
}

export function parseThemeColor(input: string): string | null {
  let value = input.trim().toLowerCase()
  if (!value.startsWith('#')) value = `#${value}`
  if (/^#[0-9a-f]{3}$/.test(value)) value = `#${[...value.slice(1)].map(digit => digit + digit).join('')}`
  return isThemeColor(value) ? value : null
}

export function alphaOf(value: string | undefined | null): number {
  if (!isThemeColor(value) || value!.length === 7) return 1
  return Number.parseInt(value!.slice(7, 9), 16) / 255
}

export function withAlpha(color: string, alpha: number): string {
  const rgb = color.slice(0, 7).toLowerCase()
  if (alpha >= 1) return rgb
  return rgb + Math.round(Math.max(0, alpha) * 255).toString(16).padStart(2, '0')
}

function cssColorToHex(value: string, probe: HTMLElement): string {
  probe.style.color = ''
  probe.style.color = value.trim()

  const channels = getComputedStyle(probe).color.match(/[\d.]+/g)?.map(Number) ?? [0, 0, 0]
  const [red = 0, green = 0, blue = 0, alpha = 1] = channels
  const hex = [red, green, blue].map(channel => Math.round(channel).toString(16).padStart(2, '0')).join('')

  return withAlpha(`#${hex}`, alpha)
}

export function readPalette(base: string): Record<ThemeToken, string> {
  const root = document.documentElement
  const theme = root.dataset.theme
  const accent = root.dataset.accent
  const inline = THEME_TOKENS.map(token => [token, root.style.getPropertyValue(`--cast-${token}`)] as const)

  const probe = document.createElement('span')
  probe.style.display = 'none'
  document.body.appendChild(probe)

  root.dataset.theme = base
  delete root.dataset.accent
  for (const token of THEME_TOKENS) root.style.removeProperty(`--cast-${token}`)

  const style = getComputedStyle(root)
  const palette = Object.fromEntries(
    THEME_TOKENS.map(token => [token, cssColorToHex(style.getPropertyValue(`--cast-${token}`), probe)]),
  ) as Record<ThemeToken, string>

  probe.remove()
  if (theme) root.dataset.theme = theme
  if (accent) root.dataset.accent = accent
  for (const [token, value] of inline) {
    if (value) root.style.setProperty(`--cast-${token}`, value)
  }

  return palette
}

export function isCustomAccent(value: string | undefined | null): boolean {
  return value != null && HEX.test(value)
}

export function accentOf(value: string | undefined | null): string {
  if (isCustomAccent(value)) return value!.toLowerCase()
  return ACCENTS.some(accent => accent.value === value) ? value! : DEFAULT_ACCENT
}

function luminanceOf(hex: string): number {
  const channel = (offset: number) => {
    const value = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

export function onAccentOf(hex: string): string {
  return luminanceOf(hex) > 0.18 ? '#0b0d10' : '#ffffff'
}
