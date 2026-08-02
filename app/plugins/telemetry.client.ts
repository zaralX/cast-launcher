import {trackEvent} from "~/composables/useTelemetry"

export default defineNuxtPlugin(nuxtApp => {
    const router = useRouter()

    let previous = ""

    router.afterEach(to => {
        const name = String(to.name ?? to.path)

        if (name === previous) return
        previous = name

        trackEvent("page_view", {name})
    })

    nuxtApp.hook("vue:error", error => {
        trackEvent("ui_crash", {message: String((error as Error)?.message ?? error)})
    })
})
