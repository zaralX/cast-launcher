import {useAppStore} from "~/stores/app"

export interface Accent {
    value: string
    label: string
    preview: string
}

export const DEFAULT_ACCENT = "sky"

export const ACCENTS: Accent[] = [
    {value: "sky", label: "Небо", preview: "#38bdf8"},
    {value: "blue", label: "Синий", preview: "#60a5fa"},
    {value: "indigo", label: "Индиго", preview: "#818cf8"},
    {value: "violet", label: "Фиолетовый", preview: "#a78bfa"},
    {value: "fuchsia", label: "Фуксия", preview: "#e879f9"},
    {value: "rose", label: "Розовый", preview: "#fb7185"},
    {value: "orange", label: "Оранжевый", preview: "#fb923c"},
    {value: "amber", label: "Янтарь", preview: "#fbbf24"},
    {value: "lime", label: "Лайм", preview: "#a3e635"},
    {value: "emerald", label: "Изумруд", preview: "#34d399"},
    {value: "teal", label: "Бирюза", preview: "#2dd4bf"},
    {value: "cyan", label: "Циан", preview: "#22d3ee"}
]

export function accentOf(value: string | undefined | null): string {
    return ACCENTS.some(accent => accent.value === value) ? value! : DEFAULT_ACCENT
}

/**
 * Основной цвет приложения из конфига.
 * Меняет палитру `primary` у Nuxt UI, от неё же берётся `--cast-acid`.
 */
export function useAccent() {
    const store = useAppStore()
    return computed(() => accentOf(store.config?.launcher.accent))
}

/**
 * Компактный режим интерфейса.
 * Помимо этого флага на `<html>` висит класс `compact` — можно цеплять из CSS.
 */
export function useCompact() {
    const store = useAppStore()
    return computed(() => store.config?.launcher.compact === true)
}

/** Держит DOM и Nuxt UI в согласии с настройками внешнего вида. Вызывается один раз в `app.vue`. */
export function useAppearance() {
    const appConfig = useAppConfig()
    const accent = useAccent()
    const compact = useCompact()

    watchEffect(() => {
        appConfig.ui.colors.primary = accent.value
    })

    if (import.meta.client) {
        watchEffect(() => {
            document.documentElement.classList.toggle("compact", compact.value)
        })
    }

    return {accent, compact}
}
