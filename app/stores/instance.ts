import {defineStore} from 'pinia'
import type {GameLogLine, Instance, InstallSnapshot, RunningGame} from '~/types/instance'
import {isTerminalStage} from '~/types/instance'
import {call, type InstanceUpdate, type NewInstance} from '~/types/backend'
import {captureError} from '~/composables/useErrorHandler'
import {LauncherError} from '~/types/error'

const LOG_BUFFER = 3000

export const useInstanceStore = defineStore('instance', {
    state: () => ({
        instances: [] as Instance[],
        installs: [] as InstallSnapshot[],
        running: [] as RunningGame[],
        logs: {} as Record<string, GameLogLine[]>
    }),
    getters: {
        getInstance: (state) => (id: string) => state.instances.find(instance => instance.id === id),
        getInstall: (state) => (id: string) => state.installs.find(install => install.instanceId === id),
        isRunning: (state) => (id: string) => state.running.some(game => game.instanceId === id),
        getLogs: (state) => (id: string): GameLogLine[] => state.logs[id] ?? []
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

            this.logs[game.instanceId] = []
        },

        applyGameLog(instanceId: string, line: GameLogLine) {
            const lines = this.logs[instanceId] ??= []

            lines.push(line)

            if (lines.length > LOG_BUFFER) lines.splice(0, lines.length - LOG_BUFFER)
        },

        clearLogs(instanceId: string) {
            this.logs[instanceId] = []
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

        async updateInstance(instanceId: string, update: InstanceUpdate) {
            const instance = await call("update_instance", {instanceId, update})
            const index = this.instances.findIndex(item => item.id === instanceId)

            if (index >= 0) this.instances[index] = instance

            return instance
        },

        async deleteInstance(instanceId: string) {
            await call("delete_instance", {instanceId})
            delete this.logs[instanceId]
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
