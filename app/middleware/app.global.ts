import {useAppStore} from "~/stores/app";
import {storeToRefs} from "pinia";

export default defineNuxtRouteMiddleware(async (to, from) => {
    if (to.path === '/') return;

    const store = useAppStore();
    const {hasConfig} = storeToRefs(store)

    if (!hasConfig.value) {
        return navigateTo("/", { redirectCode: 301 })
    }
})