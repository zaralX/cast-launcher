import type {Instance} from "~/types/instance"
import {call, type InstanceDir} from "~/types/backend"

export type InstanceState = "running" | "installing" | "ready" | "absent"

export const INSTANCE_DIR_KEYS: Record<InstanceDir, string> = {
    root: "instance.dir.root",
    minecraft: "instance.dir.minecraft",
    logs: "instance.dir.logs"
}

export function useInstanceActions() {
    const {t} = useI18n()
    const store = useInstanceStore()

    const stateOf = (instance: Instance): InstanceState => {
        if (store.isRunning(instance.id)) return "running"
        if (store.getInstall(instance.id)) return "installing"
        return instance.installed ? "ready" : "absent"
    }

    const installOf = (id: string) => store.getInstall(id)

    const play = (id: string) => safeRun(
        () => store.playInstance(id),
        {context: {instanceId: id, action: t("instance.context.play")}}
    )

    const stop = (id: string) => safeRun(
        () => store.stopInstance(id),
        {context: {instanceId: id, action: t("instance.context.stop")}}
    )

    const install = (id: string) => safeRun(
        () => store.installInstance(id),
        {context: {instanceId: id, action: t("instance.context.install")}}
    )

    const cancelInstall = (id: string) => safeRun(
        () => store.abortInstall(id),
        {context: {instanceId: id, action: t("instance.context.cancel")}}
    )

    const openDir = (id: string, target: InstanceDir) => safeRun(
        () => call("open_instance_dir", {instanceId: id, target}),
        {context: {instanceId: id, action: t(INSTANCE_DIR_KEYS[target])}}
    )

    const remove = (id: string) => attempt(
        () => store.deleteInstance(id),
        {context: {instanceId: id, action: t("instance.remove_action")}}
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
