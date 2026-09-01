import {useAppStore} from "~/stores/app";
import {storeToRefs} from "pinia";

const WELCOME = "/welcome"

export default defineNuxtRouteMiddleware(async (to, from) => {
    if (to.path === '/') return;

    const store = useAppStore();
    const {hasConfig, needsOnboarding} = storeToRefs(store)

    if (!hasConfig.value) {
        return navigateTo("/", { redirectCode: 301 })
    }

    if (needsOnboarding.value) {
        return to.path === WELCOME ? undefined : navigateTo(WELCOME)
    }

    if (to.path === WELCOME) {
        return navigateTo("/main")
    }
})
