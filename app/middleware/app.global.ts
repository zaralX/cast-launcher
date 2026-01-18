import {useAppStore} from "~/stores/app";
import {storeToRefs} from "pinia";

export default defineNuxtRouteMiddleware(async () => {
    const store = useAppStore();
    const {hasConfig} = storeToRefs(store)

    if (!hasConfig.value) {
        navigateTo("/")
    }
})