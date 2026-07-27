import {useErrorStore} from "~/stores/error";

export default defineNuxtPlugin((nuxtApp) => {
    const report = (raw: unknown, context: Record<string, unknown>) => {
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
