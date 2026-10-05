import type { UiText } from '~/types/backend'
import type { Attempt, CommandError, ErrorCode, ErrorContext, ErrorSeverity, LauncherErrorOptions, ReportOptions } from '~/types/error'
import { ERROR_CATALOG } from '~/types/error'

function translate(key: string): string {
  return useNuxtApp().$i18n.t(key)
}

export function errorTitle(code: ErrorCode): string {
  return translate(`${ERROR_CATALOG[code].key}.title`)
}

export function errorHint(code: ErrorCode): string | undefined {
  const key = `${ERROR_CATALOG[code].key}.hint`
  const i18n = useNuxtApp().$i18n

  return i18n.te(key) ? i18n.t(key) : undefined
}

export class LauncherError extends Error {
  readonly code: ErrorCode
  readonly text?: UiText
  readonly details?: string
  readonly context: ErrorContext

  constructor(code: ErrorCode, options: LauncherErrorOptions = {}) {
    super(options.message ?? options.text?.key ?? errorTitle(code), { cause: options.cause })
    this.name = 'LauncherError'
    this.code = code
    this.text = options.text
    this.details = options.details
    this.context = options.context ?? {}
  }

  get title(): string {
    return errorTitle(this.code)
  }

  get reason(): string | undefined {
    return this.text ? uiText(this.text) : undefined
  }

  get hint(): string | undefined {
    return errorHint(this.code)
  }

  get severity(): ErrorSeverity {
    return ERROR_CATALOG[this.code].severity
  }

  get icon(): string {
    return ERROR_CATALOG[this.code].icon
  }

  withContext(context: ErrorContext): this {
    Object.assign(this.context, context)
    return this
  }

  toReport(): string {
    const lines = [
      `[${this.code}] ${this.title}`,
      this.reason ?? null,
      !this.text && this.message !== this.title ? this.message : null,
      this.details ? `\n${translate('error.report.details')}\n${this.details}` : null,
    ].filter(Boolean)

    const context = Object.entries(this.context).filter(([, v]) => v !== undefined)
    if (context.length) {
      lines.push(`\n${translate('error.report.context')}`)
      for (const [key, value] of context) {
        lines.push(`  ${key}: ${stringify(value)}`)
      }
    }

    return lines.join('\n')
  }
}

function isErrorCode(value: unknown): value is ErrorCode {
  return typeof value === 'string' && value in ERROR_CATALOG
}

function asCommandError(raw: unknown): CommandError | null {
  if (typeof raw !== 'object' || raw === null) return null
  const candidate = raw as Partial<CommandError>
  if (!isErrorCode(candidate.code) || typeof candidate.text?.key !== 'string') return null
  return candidate as CommandError
}

const MESSAGE_PATTERNS: [RegExp, ErrorCode][] = [
  [/HASH_MISMATCH/, 'HASH_MISMATCH'],
  [/DOWNLOAD_FAILED/, 'DOWNLOAD_FAILED'],
  [/INSTALL_ABORTED/, 'INSTALL_ABORTED'],
  [/failed to fetch|networkerror|econnrefused|econnreset|enotfound|etimedout|timed? ?out/i, 'NETWORK'],
  [/forbidden path|not allowed on the configured scope|permission denied|eacces|eperm|ebusy/i, 'FS_ERROR'],
  [/no such file|enoent|os error 2\b/i, 'FS_ERROR'],
  [/invalid zip|zip entry|not a valid archive/i, 'ARCHIVE_INVALID'],
  [/is not valid json|unexpected token .* json|unexpected end of json/i, 'MANIFEST_INVALID'],
]

function classifyMessage(message: string): ErrorCode | null {
  for (const [pattern, code] of MESSAGE_PATTERNS) {
    if (pattern.test(message)) return code
  }
  return null
}

function stringify(value: unknown): string {
  if (typeof value === 'string') return value
  try {
    return JSON.stringify(value) ?? String(value)
  }
  catch {
    return String(value)
  }
}

function describeCause(error: Error): string | undefined {
  const parts: string[] = []

  const status = (error as { statusCode?: number, status?: number }).statusCode
    ?? (error as { status?: number }).status
  if (typeof status === 'number') parts.push(`HTTP ${status}`)

  if (error.cause instanceof Error) {
    parts.push(`${error.cause.name}: ${error.cause.message}`)
  }
  else if (error.cause !== undefined) {
    parts.push(stringify(error.cause))
  }

  if (error.stack) parts.push(error.stack)

  return parts.length ? parts.join('\n') : undefined
}

export function toLauncherError(
  raw: unknown,
  fallback: ErrorCode = 'UNKNOWN',
  context: ErrorContext = {},
): LauncherError {
  if (raw instanceof LauncherError) return raw.withContext(context)

  const command = asCommandError(raw)
  if (command) {
    return new LauncherError(command.code as ErrorCode, {
      text: command.text,
      details: command.details,
      context,
      cause: raw,
    })
  }

  if (raw instanceof Error) {
    const status = (raw as { statusCode?: number }).statusCode
    const code = classifyMessage(raw.message)
      ?? (typeof status === 'number' ? 'NETWORK' : null)
      ?? fallback

    return new LauncherError(code, {
      message: raw.message,
      details: describeCause(raw),
      context,
      cause: raw,
    })
  }

  if (typeof raw === 'string') {
    return new LauncherError(classifyMessage(raw) ?? fallback, {
      message: raw,
      context,
      cause: raw,
    })
  }

  return new LauncherError(fallback, {
    details: stringify(raw),
    context,
    cause: raw,
  })
}

export function captureError(raw: unknown, options: ReportOptions = {}): LauncherError {
  return useErrorStore().report(raw, options)
}

export async function safeRun<T>(
  operation: () => Promise<T>,
  options: ReportOptions = {},
): Promise<T | undefined> {
  try {
    return await operation()
  }
  catch (e) {
    captureError(e, options)
    return undefined
  }
}

export async function attempt<T>(
  operation: () => Promise<T>,
  options: ReportOptions = {},
): Promise<Attempt<T>> {
  try {
    return { ok: true, value: await operation() }
  }
  catch (e) {
    return { ok: false, error: captureError(e, options) }
  }
}
