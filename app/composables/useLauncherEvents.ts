import {call, onLauncherEvent} from "~/types/backend"
import {captureError} from "~/composables/useErrorHandler"
import {LauncherError} from "~/types/error"
import {useAppStore} from "~/stores/app"
import {useInstanceStore} from "~/stores/instance"
import {useAccountStore} from "~/stores/account"

let started: Promise<void> | null = null

export function useLauncherEvents(): Promise<void> {
    if (started) return started

    started = connect().catch(error => {
        started = null
        throw error
    })

    return started
}

async function connect() {
    const appStore = useAppStore()
    const instanceStore = useInstanceStore()
    const accountStore = useAccountStore()

    const bootstrap = await call("bootstrap")

    appStore.applyBootstrap(bootstrap.config, bootstrap.paths)
    accountStore.applyBootstrap(bootstrap.accounts)
    instanceStore.applyBootstrap(bootstrap.instances, bootstrap.installs, bootstrap.running)

    await onLauncherEvent(event => {
        switch (event.type) {
            case "install":
                instanceStore.applyInstall(event)
                break
            case "instances":
                instanceStore.applyInstances(event.instances)
                break
            case "gameStarted":
                instanceStore.applyGameStarted(event.game)
                break
            case "gameStatus":
                instanceStore.applyGameStatus(event.runId, event.status)
                break
            case "gameExited":
                instanceStore.applyGameExited(event.runId, event.code, event.logTail)
                break
            case "gameLog":
                instanceStore.applyGameLog(event.instanceId, {
                    runId: event.runId,
                    line: event.line,
                    isError: event.isError
                })
                break
            case "launchFailed":
                captureError(new LauncherError("LAUNCH_FAILED", {
                    message: event.error,
                    context: {instanceId: event.instanceId, instanceName: event.instanceName}
                }))
                break
        }
    })
}
