import {onBeforeRouteLeave, onBeforeRouteUpdate} from "vue-router"
import type {MaybeRefOrGetter} from "vue"

export interface UnsavedChangesOptions {
    dirty: MaybeRefOrGetter<boolean>
    canSave?: MaybeRefOrGetter<boolean>
    save: () => Promise<boolean | void> | boolean | void
    discard?: () => void
}

export function useUnsavedChanges(options: UnsavedChangesOptions) {
    const open = ref(false)
    const saving = ref(false)

    const dirty = computed(() => toValue(options.dirty))
    const canSave = computed(() => toValue(options.canSave ?? true))

    let resolveLeave: ((leave: boolean) => void) | null = null

    function settle(leave: boolean) {
        const resolve = resolveLeave

        resolveLeave = null
        open.value = false

        resolve?.(leave)
    }

    function intercept() {
        if (!dirty.value) return true
        if (open.value) return false

        open.value = true

        return new Promise<boolean>(resolve => {
            resolveLeave = resolve
        })
    }

    onBeforeRouteLeave(intercept)
    onBeforeRouteUpdate(intercept)

    watch(open, value => {
        if (!value) settle(false)
    })

    onScopeDispose(() => settle(false))

    async function save() {
        if (saving.value || !canSave.value) return

        saving.value = true

        try {
            const ok = await options.save()
            if (ok === false) return
        } finally {
            saving.value = false
        }

        settle(true)
    }

    function discard() {
        options.discard?.()
        settle(true)
    }

    const cancel = () => settle(false)

    return reactive({open, saving, dirty, canSave, save, discard, cancel})
}

export type UnsavedChanges = ReturnType<typeof useUnsavedChanges>
