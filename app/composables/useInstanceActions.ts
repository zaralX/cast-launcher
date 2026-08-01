import type {Instance} from "~/types/instance"
import {call, type InstanceDir} from "~/types/backend"

export type InstanceState = "running" | "installing" | "ready" | "absent"

export const INSTANCE_DIR_LABELS: Record<InstanceDir, string> = {
    root: "Папка сборки",
    minecraft: "Папка .minecraft",
    logs: "Папка логов"
}

export function useInstanceActions() {
    const store = useInstanceStore()

    const stateOf = (instance: Instance): InstanceState => {
        if (store.isRunning(instance.id)) return "running"
        if (store.getInstall(instance.id)) return "installing"
        return instance.installed ? "ready" : "absent"
    }

    const installOf = (id: string) => store.getInstall(id)

    const play = (id: string) => safeRun(
        () => store.playInstance(id),
        {context: {instanceId: id, action: "Запуск сборки"}}
    )

    const stop = (id: string) => safeRun(
        () => store.stopInstance(id),
        {context: {instanceId: id, action: "Остановка сборки"}}
    )

    const install = (id: string) => safeRun(
        () => store.installInstance(id),
        {context: {instanceId: id, action: "Установка сборки"}}
    )

    const cancelInstall = (id: string) => safeRun(
        () => store.abortInstall(id),
        {context: {instanceId: id, action: "Отмена установки"}}
    )

    const openDir = (id: string, target: InstanceDir) => safeRun(
        () => call("open_instance_dir", {instanceId: id, target}),
        {context: {instanceId: id, action: INSTANCE_DIR_LABELS[target]}}
    )

    const remove = (id: string) => attempt(
        () => store.deleteInstance(id),
        {context: {instanceId: id, action: "Удаление сборки"}}
    )

    const primary = (instance: Instance) => {
        switch (stateOf(instance)) {
            case "ready":
                return play(instance.id)
            case "absent":
                return install(instance.id)
            default:
                return Promise.resolve()
        }
    }

    return {stateOf, installOf, play, stop, install, cancelInstall, openDir, remove, primary}
}
