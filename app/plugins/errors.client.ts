import {useErrorStore} from "~/stores/error";

const NOISE = [
    "ResizeObserver loop completed with undelivered notifications",
    "ResizeObserver loop limit exceeded"
]

function isNoise(raw: unknown): boolean {
    const message = typeof raw === "string" ? raw : (raw as { message?: unknown })?.message

    return typeof message === "string" && NOISE.some(pattern => message.includes(pattern))
}

export default defineNuxtPlugin((nuxtApp) => {
    const report = (raw: unknown, context: Record<string, unknown>) => {
        if (isNoise(raw)) return

        try {
            useErrorStore().report(raw, {context})
        } catch (e) {
            console.error("Failed to report error", e, raw)
        }
    }

    nuxtApp.vueApp.config.errorHandler = (error, _instance, info) => {
        report(error, {source: "vue", info})
    }

    window.addEventListener("unhandledrejection", (event) => {
        report(event.reason, {source: "unhandledrejection"})
    })

    window.addEventListener("error", (event) => {
        report(event.error ?? event.message, {source: "window.error"})
    })
})
