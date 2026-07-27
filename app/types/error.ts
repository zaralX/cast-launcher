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
    title: string
    hint?: string
    severity: ErrorSeverity
    icon: string
}

export const ERROR_CATALOG: Record<ErrorCode, ErrorDefinition> = {
    NETWORK: {
        title: "Нет связи с сервером",
        hint: "Проверьте интернет-соединение и попробуйте ещё раз.",
        severity: "error",
        icon: "i-lucide-wifi-off"
    },
    DOWNLOAD_FAILED: {
        title: "Не удалось скачать файл",
        hint: "Сервер раздачи недоступен или соединение оборвалось. Повторите установку.",
        severity: "error",
        icon: "i-lucide-cloud-download"
    },
    HASH_MISMATCH: {
        title: "Файл скачался повреждённым",
        hint: "Контрольная сумма не совпала. Повторите установку — файл будет скачан заново.",
        severity: "error",
        icon: "i-lucide-file-x"
    },
    FS_ERROR: {
        title: "Ошибка доступа к файлам",
        hint: "Проверьте, что папка лаунчера существует и не занята другой программой.",
        severity: "error",
        icon: "i-lucide-folder-x"
    },
    ARCHIVE_INVALID: {
        title: "Повреждённый архив",
        hint: "Не удалось распаковать jar. Удалите кэш лаунчера и повторите установку.",
        severity: "error",
        icon: "i-lucide-file-archive"
    },
    MANIFEST_INVALID: {
        title: "Некорректный ответ сервера",
        hint: "Mojang, Fabric или Forge вернули неожиданные данные.",
        severity: "error",
        icon: "i-lucide-file-question"
    },
    VERSION_NOT_FOUND: {
        title: "Версия не найдена",
        hint: "Такой версии нет в манифесте. Выберите другую при создании сборки.",
        severity: "error",
        icon: "i-lucide-search-x"
    },
    JAVA_NOT_FOUND: {
        title: "Java не найдена",
        hint: "Укажите путь к Java в настройках или установите её.",
        severity: "error",
        icon: "i-lucide-coffee"
    },
    LAUNCH_FAILED: {
        title: "Не удалось запустить Minecraft",
        severity: "error",
        icon: "i-lucide-play"
    },
    FORGE_INSTALL_FAILED: {
        title: "Не удалось установить Forge",
        hint: "Официальный установщик Forge завершился с ошибкой. Подробности — в деталях.",
        severity: "error",
        icon: "i-lucide-hammer"
    },
    AUTH_FAILED: {
        title: "Ошибка входа",
        hint: "Не удалось авторизоваться через Microsoft. Попробуйте войти заново.",
        severity: "error",
        icon: "i-lucide-user-x"
    },
    AUTH_PORT_BUSY: {
        title: "Порт входа занят",
        hint: "Порт 55325 уже используется. Закройте другую копию лаунчера и попробуйте снова.",
        severity: "error",
        icon: "i-lucide-plug-zap"
    },
    AUTH_EXPIRED: {
        title: "Сессия истекла",
        hint: "Войдите в аккаунт Microsoft заново.",
        severity: "warning",
        icon: "i-lucide-clock-alert"
    },
    NO_ACCOUNT: {
        title: "Аккаунт не выбран",
        hint: "Добавьте аккаунт в настройках, прежде чем запускать игру.",
        severity: "warning",
        icon: "i-lucide-user-round-x"
    },
    CONFIG_ERROR: {
        title: "Ошибка конфигурации",
        hint: "Файл настроек повреждён. Лаунчер продолжит работу со значениями по умолчанию.",
        severity: "warning",
        icon: "i-lucide-settings-2"
    },
    UPDATE_FAILED: {
        title: "Обновление не установлено",
        hint: "Лаунчер продолжит работу на текущей версии.",
        severity: "warning",
        icon: "i-lucide-download"
    },
    INSTALL_ABORTED: {
        title: "Установка прервана",
        severity: "info",
        icon: "i-lucide-circle-stop"
    },
    UNKNOWN: {
        title: "Непредвиденная ошибка",
        hint: "Скопируйте детали и приложите их к сообщению об ошибке.",
        severity: "error",
        icon: "i-lucide-triangle-alert"
    }
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
        super(options.message ?? ERROR_CATALOG[code].title, { cause: options.cause })
        this.name = "LauncherError"
        this.code = code
        this.details = options.details
        this.context = options.context ?? {}
    }

    get title(): string {
        return ERROR_CATALOG[this.code].title
    }

    get hint(): string | undefined {
        return ERROR_CATALOG[this.code].hint
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
            this.details ? `\nДетали:\n${this.details}` : null
        ].filter(Boolean)

        const context = Object.entries(this.context).filter(([, v]) => v !== undefined)
        if (context.length) {
            lines.push("\nКонтекст:")
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

    // ofetch/FetchError несёт HTTP-статус отдельно от текста
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
