import type {MaybeRefOrGetter} from "vue"
import type {Instance} from "~/types/instance"

const TICK = 15_000

const now = ref(Date.now())

let timer: ReturnType<typeof setInterval> | null = null
let listeners = 0

function useClock() {
    onMounted(() => {
        listeners++

        if (!timer) {
            now.value = Date.now()
            timer = setInterval(() => now.value = Date.now(), TICK)
        }
    })

    onUnmounted(() => {
        listeners--

        if (listeners <= 0 && timer) {
            clearInterval(timer)
            timer = null
        }
    })

    return now
}

export function usePlaytime(source: MaybeRefOrGetter<Instance | null | undefined>) {
    const instanceStore = useInstanceStore()
    const clock = useClock()

    const session = computed(() => {
        const instance = toValue(source)
        if (!instance) return 0

        const game = instanceStore.running.find(running => running.instanceId === instance.id)
        if (!game) return 0

        return Math.max(0, Math.floor((clock.value - game.startedAt) / 1000))
    })

    const total = computed(() => (toValue(source)?.playtime?.totalSeconds ?? 0) + session.value)
    const last = computed(() => session.value || (toValue(source)?.playtime?.lastSeconds ?? 0))

    return {session, total, last}
}
