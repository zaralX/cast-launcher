import {defineStore} from 'pinia'
import type {Instance, InstallSnapshot, RunningGame} from '~/types/instance'
import {isTerminalStage} from '~/types/instance'
import {call, type NewInstance} from '~/types/backend'
import {captureError} from '~/composables/useErrorHandler'
import {LauncherError} from '~/types/error'

export const useInstanceStore = defineStore('instance', {
    state: () => ({
        instances: [] as Instance[],
        installs: [] as InstallSnapshot[],
        running: [] as RunningGame[]
    }),
    getters: {
        getInstance: (state) => (id: string) => state.instances.find(instance => instance.id === id),
        getInstall: (state) => (id: string) => state.installs.find(install => install.instanceId === id),
        isRunning: (state) => (id: string) => state.running.some(game => game.instanceId === id)
    },
    actions: {
        applyBootstrap(instances: Instance[], installs: InstallSnapshot[], running: RunningGame[]) {
            this.instances = instances
            this.installs = installs
            this.running = running
        },

        applyInstances(instances: Instance[]) {
            this.instances = instances
        },

        applyInstall(snapshot: InstallSnapshot) {
            const index = this.installs.findIndex(install => install.instanceId === snapshot.instanceId)

            if (isTerminalStage(snapshot.stage)) {
                if (index >= 0) this.installs.splice(index, 1)
                if (snapshot.stage === "failed" && snapshot.error) {
                    captureError(new LauncherError("UNKNOWN", {
                        message: snapshot.error,
                        context: {instanceId: snapshot.instanceId, instanceName: snapshot.instanceName}
                    }))
                }
                return
            }

            if (index >= 0) this.installs[index] = snapshot
            else this.installs.push(snapshot)
        },

        applyGameStarted(game: RunningGame) {
            if (!this.running.some(running => running.runId === game.runId)) {
                this.running.push(game)
            }
        },

        applyGameStatus(runId: string, status: RunningGame["status"]) {
            const game = this.running.find(running => running.runId === runId)
            if (game) game.status = status
        },

        applyGameExited(runId: string, code: number | null, logTail?: string) {
            const index = this.running.findIndex(running => running.runId === runId)
            if (index < 0) return

            const [game] = this.running.splice(index, 1)
            if (!game || code === 0) return

            captureError(new LauncherError("LAUNCH_FAILED", {
                message: `Minecraft завершился с кодом ${code ?? "неизвестно"}`,
                details: logTail,
                context: {instanceId: game.instanceId, instanceName: game.instanceName, exitCode: code}
            }))
        },

        async createInstance(instance: NewInstance) {
            return await call("create_instance", {instance})
        },

        async deleteInstance(instanceId: string) {
            await call("delete_instance", {instanceId})
        },

        async installInstance(instanceId: string) {
            const snapshot = await call("install_instance", {instanceId})

            if (!this.getInstall(instanceId)) this.applyInstall(snapshot)
        },

        async abortInstall(instanceId: string) {
            const install = this.getInstall(instanceId)
            if (install) install.aborting = true

            await call("cancel_install", {instanceId})
        },

        async runInstance(instanceId: string) {
            const game = await call("launch_instance", {instanceId})
            this.applyGameStarted(game)
        },

        async stopInstance(instanceId: string) {
            await call("stop_instance", {instanceId})
        }
    }
})
