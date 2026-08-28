export type ErrorSeverity = "error" | "warning" | "info"

export type ErrorCode =
    | "NETWORK"
    | "DOWNLOAD_FAILED"
    | "HASH_MISMATCH"
    | "FS_ERROR"
    | "ARCHIVE_INVALID"
    | "MANIFEST_INVALID"
    | "VERSION_NOT_FOUND"
    | "JAVA_NOT_FOUND"
    | "LAUNCH_FAILED"
    | "FORGE_INSTALL_FAILED"
    | "AUTH_FAILED"
    | "AUTH_PORT_BUSY"
    | "AUTH_EXPIRED"
    | "NO_ACCOUNT"
    | "CONFIG_ERROR"
    | "UPDATE_FAILED"
    | "INSTALL_ABORTED"
    | "UNKNOWN"

export interface ErrorContext {
    instanceId?: string
    instanceName?: string
    url?: string
    path?: string
    stage?: string
    [key: string]: unknown
}

interface ErrorDefinition {
    key: string
    severity: ErrorSeverity
    icon: string
}

export const ERROR_CATALOG: Record<ErrorCode, ErrorDefinition> = {
    NETWORK: {key: "error.network", severity: "error", icon: "i-lucide-wifi-off"},
    DOWNLOAD_FAILED: {key: "error.download_failed", severity: "error", icon: "i-lucide-cloud-download"},
    HASH_MISMATCH: {key: "error.hash_mismatch", severity: "error", icon: "i-lucide-file-x"},
    FS_ERROR: {key: "error.fs", severity: "error", icon: "i-lucide-folder-x"},
    ARCHIVE_INVALID: {key: "error.archive_invalid", severity: "error", icon: "i-lucide-file-archive"},
    MANIFEST_INVALID: {key: "error.manifest_invalid", severity: "error", icon: "i-lucide-file-question"},
    VERSION_NOT_FOUND: {key: "error.version_not_found", severity: "error", icon: "i-lucide-search-x"},
    JAVA_NOT_FOUND: {key: "error.java_not_found", severity: "error", icon: "i-lucide-coffee"},
    LAUNCH_FAILED: {key: "error.launch_failed", severity: "error", icon: "i-lucide-play"},
    FORGE_INSTALL_FAILED: {key: "error.forge_install_failed", severity: "error", icon: "i-lucide-hammer"},
    AUTH_FAILED: {key: "error.auth_failed", severity: "error", icon: "i-lucide-user-x"},
    AUTH_PORT_BUSY: {key: "error.auth_port_busy", severity: "error", icon: "i-lucide-plug-zap"},
    AUTH_EXPIRED: {key: "error.auth_expired", severity: "warning", icon: "i-lucide-clock-alert"},
    NO_ACCOUNT: {key: "error.no_account", severity: "warning", icon: "i-lucide-user-round-x"},
    CONFIG_ERROR: {key: "error.config", severity: "warning", icon: "i-lucide-settings-2"},
    UPDATE_FAILED: {key: "error.update_failed", severity: "warning", icon: "i-lucide-download"},
    INSTALL_ABORTED: {key: "error.install_aborted", severity: "info", icon: "i-lucide-circle-stop"},
    UNKNOWN: {key: "error.unknown", severity: "error", icon: "i-lucide-triangle-alert"}
}

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

export interface LauncherErrorOptions {
    message?: string
    details?: string
    context?: ErrorContext
    cause?: unknown
}

export class LauncherError extends Error {
    readonly code: ErrorCode
    readonly details?: string
    readonly context: ErrorContext

    constructor(code: ErrorCode, options: LauncherErrorOptions = {}) {
        super(options.message ?? errorTitle(code), { cause: options.cause })
        this.name = "LauncherError"
        this.code = code
        this.details = options.details
        this.context = options.context ?? {}
    }

    get title(): string {
        return errorTitle(this.code)
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
            this.message !== this.title ? this.message : null,
            this.details ? `\n${translate("error.report.details")}\n${this.details}` : null
        ].filter(Boolean)

        const context = Object.entries(this.context).filter(([, v]) => v !== undefined)
        if (context.length) {
            lines.push(`\n${translate("error.report.context")}`)
            for (const [key, value] of context) {
                lines.push(`  ${key}: ${stringify(value)}`)
            }
        }

        return lines.join("\n")
    }
}

interface CommandError {
    code: string
    message: string
    details?: string
}

function isErrorCode(value: unknown): value is ErrorCode {
    return typeof value === "string" && value in ERROR_CATALOG
}

function asCommandError(raw: unknown): CommandError | null {
    if (typeof raw !== "object" || raw === null) return null
    const candidate = raw as Partial<CommandError>
    if (!isErrorCode(candidate.code) || typeof candidate.message !== "string") return null
    return candidate as CommandError
}

const MESSAGE_PATTERNS: [RegExp, ErrorCode][] = [
    [/HASH_MISMATCH/, "HASH_MISMATCH"],
    [/DOWNLOAD_FAILED/, "DOWNLOAD_FAILED"],
    [/INSTALL_ABORTED/, "INSTALL_ABORTED"],
    [/failed to fetch|networkerror|econnrefused|econnreset|enotfound|etimedout|timed? ?out/i, "NETWORK"],
    [/forbidden path|not allowed on the configured scope|permission denied|eacces|eperm|ebusy/i, "FS_ERROR"],
    [/no such file|enoent|os error 2\b/i, "FS_ERROR"],
    [/invalid zip|zip entry|not a valid archive/i, "ARCHIVE_INVALID"],
    [/is not valid json|unexpected token .* json|unexpected end of json/i, "MANIFEST_INVALID"]
]

function classifyMessage(message: string): ErrorCode | null {
    for (const [pattern, code] of MESSAGE_PATTERNS) {
        if (pattern.test(message)) return code
    }
    return null
}

function stringify(value: unknown): string {
    if (typeof value === "string") return value
    try {
        return JSON.stringify(value) ?? String(value)
    } catch {
        return String(value)
    }
}

function describeCause(error: Error): string | undefined {
    const parts: string[] = []

    const status = (error as { statusCode?: number, status?: number }).statusCode
        ?? (error as { status?: number }).status
    if (typeof status === "number") parts.push(`HTTP ${status}`)

    if (error.cause instanceof Error) {
        parts.push(`${error.cause.name}: ${error.cause.message}`)
    } else if (error.cause !== undefined) {
        parts.push(stringify(error.cause))
    }

    if (error.stack) parts.push(error.stack)

    return parts.length ? parts.join("\n") : undefined
}

export function toLauncherError(
    raw: unknown,
    fallback: ErrorCode = "UNKNOWN",
    context: ErrorContext = {}
): LauncherError {
    if (raw instanceof LauncherError) return raw.withContext(context)

    const command = asCommandError(raw)
    if (command) {
        return new LauncherError(command.code as ErrorCode, {
            message: command.message,
            details: command.details,
            context,
            cause: raw
        })
    }

    if (raw instanceof Error) {
        const status = (raw as { statusCode?: number }).statusCode
        const code = classifyMessage(raw.message)
            ?? (typeof status === "number" ? "NETWORK" : null)
            ?? fallback

        return new LauncherError(code, {
            message: raw.message,
            details: describeCause(raw),
            context,
            cause: raw
        })
    }

    if (typeof raw === "string") {
        return new LauncherError(classifyMessage(raw) ?? fallback, {
            message: raw,
            context,
            cause: raw
        })
    }

    return new LauncherError(fallback, {
        details: stringify(raw),
        context,
        cause: raw
    })
}
