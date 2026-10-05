import type { UiText } from '~/types/backend'
import type { LauncherError } from '~/utils/error'

export type ErrorSeverity = 'error' | 'warning' | 'info'

export type ErrorCode
  = | 'NETWORK'
    | 'DOWNLOAD_FAILED'
    | 'HASH_MISMATCH'
    | 'FS_ERROR'
    | 'ARCHIVE_INVALID'
    | 'MANIFEST_INVALID'
    | 'VERSION_NOT_FOUND'
    | 'JAVA_NOT_FOUND'
    | 'LAUNCH_FAILED'
    | 'FORGE_INSTALL_FAILED'
    | 'AUTH_FAILED'
    | 'AUTH_PORT_BUSY'
    | 'AUTH_EXPIRED'
    | 'NO_ACCOUNT'
    | 'CONFIG_ERROR'
    | 'UPDATE_FAILED'
    | 'INSTALL_ABORTED'
    | 'INVALID_INPUT'
    | 'CONFLICT'
    | 'NOT_FOUND'
    | 'UNSUPPORTED'
    | 'UNKNOWN'

export interface ErrorContext {
  instanceId?: string
  instanceName?: string
  url?: string
  path?: string
  stage?: string
  [key: string]: unknown
}

export interface ErrorDefinition {
  key: string
  severity: ErrorSeverity
  icon: string
}

export const ERROR_CATALOG: Record<ErrorCode, ErrorDefinition> = {
  NETWORK: { key: 'error.network', severity: 'error', icon: 'i-lucide-wifi-off' },
  DOWNLOAD_FAILED: { key: 'error.download_failed', severity: 'error', icon: 'i-lucide-cloud-download' },
  HASH_MISMATCH: { key: 'error.hash_mismatch', severity: 'error', icon: 'i-lucide-file-x' },
  FS_ERROR: { key: 'error.fs', severity: 'error', icon: 'i-lucide-folder-x' },
  ARCHIVE_INVALID: { key: 'error.archive_invalid', severity: 'error', icon: 'i-lucide-file-archive' },
  MANIFEST_INVALID: { key: 'error.manifest_invalid', severity: 'error', icon: 'i-lucide-file-question' },
  VERSION_NOT_FOUND: { key: 'error.version_not_found', severity: 'error', icon: 'i-lucide-search-x' },
  JAVA_NOT_FOUND: { key: 'error.java_not_found', severity: 'error', icon: 'i-lucide-coffee' },
  LAUNCH_FAILED: { key: 'error.launch_failed', severity: 'error', icon: 'i-lucide-play' },
  FORGE_INSTALL_FAILED: { key: 'error.forge_install_failed', severity: 'error', icon: 'i-lucide-hammer' },
  AUTH_FAILED: { key: 'error.auth_failed', severity: 'error', icon: 'i-lucide-user-x' },
  AUTH_PORT_BUSY: { key: 'error.auth_port_busy', severity: 'error', icon: 'i-lucide-plug-zap' },
  AUTH_EXPIRED: { key: 'error.auth_expired', severity: 'warning', icon: 'i-lucide-clock-alert' },
  NO_ACCOUNT: { key: 'error.no_account', severity: 'warning', icon: 'i-lucide-user-round-x' },
  CONFIG_ERROR: { key: 'error.config', severity: 'warning', icon: 'i-lucide-settings-2' },
  UPDATE_FAILED: { key: 'error.update_failed', severity: 'warning', icon: 'i-lucide-download' },
  INSTALL_ABORTED: { key: 'error.install_aborted', severity: 'info', icon: 'i-lucide-circle-stop' },
  INVALID_INPUT: { key: 'error.invalid_input', severity: 'warning', icon: 'i-lucide-circle-alert' },
  CONFLICT: { key: 'error.conflict', severity: 'warning', icon: 'i-lucide-hourglass' },
  NOT_FOUND: { key: 'error.not_found', severity: 'warning', icon: 'i-lucide-search-x' },
  UNSUPPORTED: { key: 'error.unsupported', severity: 'warning', icon: 'i-lucide-ban' },
  UNKNOWN: { key: 'error.unknown', severity: 'error', icon: 'i-lucide-triangle-alert' },
}

export interface LauncherErrorOptions {
  message?: string
  text?: UiText
  details?: string
  context?: ErrorContext
  cause?: unknown
}

export interface ErrorEntry {
  id: string
  at: number
  code: ErrorCode
  severity: ErrorSeverity
  icon: string
  title: string
  message: string
  reason?: string
  hint?: string
  details?: string
  context: ErrorContext
  report: string
  count: number
}

export interface ReportOptions {
  code?: ErrorCode
  context?: ErrorContext
  toast?: boolean
}

export interface CommandError {
  code: string
  text: UiText
  details?: string
}

export type Attempt<T>
  = | { ok: true, value: T }
    | { ok: false, error: LauncherError }
