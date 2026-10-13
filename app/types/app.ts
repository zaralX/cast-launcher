export interface AppUpdate {
  version: string
  notes?: string
  date?: string
  downloaded: boolean
}

export interface AppUpdateProgress {
  downloaded: number
  total: number | null
}

export interface AppConfig {
  version: number
  launcher: LauncherConfig
  java: JavaConfig
}

export interface LauncherConfig {
  language: string
  theme: string
  theme_light: string
  theme_dark: string
  themes: CustomTheme[]
  dir: string
  auto_update: boolean
  accent: string
  compact: boolean
  telemetry: boolean
  after_launch: AfterLaunch
  onboarded: boolean
}

export type AfterLaunch = 'nothing' | 'hide' | 'close'

export const AFTER_LAUNCH_VALUES: AfterLaunch[] = ['nothing', 'hide', 'close']

export type JavaMode = 'auto' | 'system' | 'manual'

export interface JavaConfig {
  java_mode: JavaMode
  java_path: string
  min_ram: number
  max_ram: number
}

export interface LauncherPaths {
  root: string
  configRoot: string
  instancesRoot: string
  icons: string
  libraries: string
  assets: string
  javaRuntimes: string
  logs: string
}

export type JavaSource
  = | 'path'
    | 'java_home'
    | 'registry'
    | 'system'
    | 'minecraft'
    | 'launcher'
    | 'manual'

export interface JavaRuntime {
  path: string
  version: string
  major: number
  vendor: string
  arch: string
  os_version: string
  is_64bit: boolean
  source: JavaSource
}

export const JAVA_SOURCE_KEYS: Record<JavaSource, string> = {
  path: 'java.source.path',
  java_home: 'java.source.java_home',
  registry: 'java.source.registry',
  system: 'java.source.system',
  minecraft: 'java.source.minecraft',
  launcher: 'java.source.launcher',
  manual: 'java.source.manual',
}

export interface Accent {
  value: string
  labelKey: string
  preview: string
}

export type ThemeToken
  = | 'frame'
    | 'sheet'
    | 'panel'
    | 'control'
    | 'hover'
    | 'raise'
    | 'line'
    | 'line-strong'
    | 'card-line'
    | 'sheet-edge'
    | 'fg'
    | 'fg-muted'
    | 'fg-faint'
    | 'accent'
    | 'on-accent'
    | 'success'
    | 'warning'
    | 'danger'
    | 'on-danger'

export interface CustomTheme {
  id: string
  name: string
  classic: boolean
  colors: Partial<Record<ThemeToken, string>>
}

export interface ThemeTokenGroup {
  labelKey: string
  tokens: {
    token: ThemeToken
    labelKey: string
    alpha?: boolean
  }[]
}

export interface ThemePreview {
  frame: string
  sheet: string
  panel: string
  accent: string
  sheetEdge?: string
  cardLine?: string
  square?: boolean
}

export interface Theme {
  value: string
  labelKey: string
  preview: ThemePreview
}
