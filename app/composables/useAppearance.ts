import {useAppStore} from "~/stores/app"

export interface Accent {
    value: string
    labelKey: string
    preview: string
}

export const DEFAULT_ACCENT = "sky"

export const ACCENTS: Accent[] = [
    {value: "sky", labelKey: "accent.sky", preview: "#38bdf8"},
    {value: "blue", labelKey: "accent.blue", preview: "#60a5fa"},
    {value: "indigo", labelKey: "accent.indigo", preview: "#818cf8"},
    {value: "violet", labelKey: "accent.violet", preview: "#a78bfa"},
    {value: "fuchsia", labelKey: "accent.fuchsia", preview: "#e879f9"},
    {value: "rose", labelKey: "accent.rose", preview: "#fb7185"},
    {value: "orange", labelKey: "accent.orange", preview: "#fb923c"},
    {value: "amber", labelKey: "accent.amber", preview: "#fbbf24"},
    {value: "lime", labelKey: "accent.lime", preview: "#a3e635"},
    {value: "emerald", labelKey: "accent.emerald", preview: "#34d399"},
    {value: "teal", labelKey: "accent.teal", preview: "#2dd4bf"},
    {value: "cyan", labelKey: "accent.cyan", preview: "#22d3ee"}
]

export function accentOf(value: string | undefined | null): string {
    return ACCENTS.some(accent => accent.value === value) ? value! : DEFAULT_ACCENT
}

export function useAccent() {
    const store = useAppStore()
    return computed(() => accentOf(store.config?.launcher.accent))
}

export function useCompact() {
    const store = useAppStore()
    return computed(() => store.config?.launcher.compact === true)
}

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
