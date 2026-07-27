import {defineStore} from 'pinia'
import type {InstallerProgress, InstallProgressView, Instance, LivingInstance} from '~/types/instance'
import {isTerminalStage} from '~/types/instance'
import {appConfigDir} from "@tauri-apps/api/path";
import {path} from "@tauri-apps/api";
import {exists, mkdir, readDir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import { v4 as uuidv4 } from "uuid";
import {InstallerBase} from "~/lib/installers/InstallerBase";
import {VanillaInstaller} from "~/lib/installers/VanillaInstaller";
import {ClientBase} from "~/lib/client/ClientBase";
import { VanillaClient } from "~/lib/client/VanillaClient";
import {FabricInstaller} from "~/lib/installers/FabricInstaller";
import {FabricClient} from "~/lib/client/FabricClient";
import { ForgeInstaller } from "~/lib/installers/ForgeInstaller";
import {ForgeClient} from "~/lib/client/ForgeClient";
import {LauncherError, toLauncherError} from "~/types/error";
import {captureError} from "~/composables/useErrorHandler";
import {listActiveDownloadJobs} from "~/lib/ParallelDownloader";

const activeInstallers = new Map<string, InstallerBase>()

export const useInstanceStore = defineStore('instance', {
    state: () => ({
        instances: [] as LivingInstance[],
        instancesDir: "",
        installs: [] as InstallProgressView[],
        runningClients: [] as ClientBase[]
    }),
    getters: {
        getInstance: (state) => {
            return (id: string) => state.instances.find((instance) => instance.id == id)
        },
        getInstall: (state) => {
            return (id: string) => state.installs.find((install) => install.instanceId == id)
        }
    },
    actions: {
        async initInstances() {
            const dataDir = await appConfigDir();
            const instancesDir = await path.join(dataDir, 'instances')

            try {
                if (!(await exists(instancesDir))) {
                    await mkdir(instancesDir, { recursive: true });
                }
            } catch (e) {
                throw toLauncherError(e, "FS_ERROR", {path: instancesDir})
            }

            this.instancesDir = instancesDir

            let instanceEntries
            try {
                instanceEntries = await readDir(instancesDir)
            } catch (e) {
                throw toLauncherError(e, "FS_ERROR", {path: instancesDir})
            }

            // Initializing all instances from /instances dir to this.instances
            const loaded: LivingInstance[] = []

            for (const instanceEntry of instanceEntries) {
                if (!instanceEntry.isDirectory) continue

                const instanceFileDir = await path.join(instancesDir, instanceEntry.name, "instance.json")
                if (!(await exists(instanceFileDir))) continue

                try {
                    const instanceConfig = JSON.parse(await readTextFile(instanceFileDir)) as Instance
                    loaded.push({
                        ...instanceConfig,
                        dir: await path.join(instancesDir, instanceEntry.name),
                        installing: false
                    })
                } catch (e) {
                    captureError(e, {
                        code: "CONFIG_ERROR",
                        context: {path: instanceFileDir, instanceName: instanceEntry.name}
                    })
                }
            }

            this.instances = loaded
        },
        async createInstance(data: Instance) {
            let instanceDir = await path.join(this.instancesDir, data.id)
            if (await exists(instanceDir)) {
                const randomId = uuidv4().split('-')[0] as string
                instanceDir = await path.join(this.instancesDir, `${data.id}-${randomId}`)
            }

            const instanceFileDir = await path.join(instanceDir, "instance.json")

            try {
                await mkdir(instanceDir, { recursive: true })
                await writeTextFile(instanceFileDir, JSON.stringify(data))
            } catch (e) {
                throw toLauncherError(e, "FS_ERROR", {path: instanceFileDir, instanceName: data.name})
            }

            await this.initInstances()
        },
        async installInstance(id: string) {
            const instance = this.getInstance(id)
            if (!instance) {
                captureError(new LauncherError("UNKNOWN", {message: `Сборка ${id} не найдена`}))
                return
            }

            if (instance.installing) return

            instance.installing = true

            this.installs.push({
                instanceId: instance.id,
                instanceName: instance.name,
                stage: "prepare",
                phase: "Подготовка",
                message: "Подготовка",
                progress: 0,
                files: [],
                startedAt: Date.now(),
                aborting: false,
                resumed: !!instance.pendingInstall
            })

            await this.setInstallPending(instance, true)

            let unsubscribe: (() => void) | null = null

            try {
                const installer = await this.createInstaller(instance)
                activeInstallers.set(instance.id, installer)

                unsubscribe = installer.onProgress(p => this.applyInstallProgress(instance.id, p))

                await installer.install()
            } catch (e) {
                captureError(e, {context: {instanceId: instance.id, instanceName: instance.name}})
            } finally {
                unsubscribe?.()
                activeInstallers.delete(instance.id)
                this.installs = this.installs.filter(install => install.instanceId !== instance.id)
                instance.installing = false
                await this.setInstallPending(instance, false)
            }
        },

        async setInstallPending(instance: LivingInstance, pending: boolean) {
            instance.pendingInstall = pending

            const instanceFileDir = await path.join(instance.dir, "instance.json")

            try {
                const data = JSON.parse(await readTextFile(instanceFileDir)) as Instance
                if (!!data.pendingInstall === pending) return
                data.pendingInstall = pending
                await writeTextFile(instanceFileDir, JSON.stringify(data))
            } catch (e) {
                console.warn("Failed to update pendingInstall flag", instanceFileDir, e)
            }
        },

        async resumeInstalls() {
            const liveJobs = await listActiveDownloadJobs()
            const hasLiveJob = (id: string) => liveJobs.some(job => job.startsWith(`${id}:`))

            for (const instance of this.instances) {
                if (instance.installed || instance.installing) continue
                if (!instance.pendingInstall && !hasLiveJob(instance.id)) continue

                this.installInstance(instance.id)
            }
        },

        applyInstallProgress(instanceId: string, p: InstallerProgress) {
            const install = this.installs.find(i => i.instanceId === instanceId)
            if (!install) return

            install.stage = p.stage

            if (p.type === "single" && p.file) {
                if (install.aborting) return

                const file = p.file
                const index = install.files.findIndex(f => f.url === file.url)

                if (file.done) {
                    if (index >= 0) install.files.splice(index, 1)
                    return
                }

                if (index >= 0) Object.assign(install.files[index]!, file)
                else install.files.push({...file})

                return
            }

            if (p.phase) install.phase = p.phase
            if (p.message) install.message = p.message
            if (typeof p.progress === "number") {
                install.progress = Math.min(1, Math.max(0, p.progress))
            }

            if (isTerminalStage(p.stage)) install.files = []
        },

        abortInstall(id: string) {
            const install = this.installs.find(i => i.instanceId === id)
            if (install) {
                if (install.aborting) return
                install.aborting = true
                install.message = "Останавливаем загрузку"
                install.files = []
            }

            activeInstallers.get(id)?.abort()
        },

        async createInstaller(instance: LivingInstance) {
            const appStore = useAppStore()
            const launcherDir = appStore?.config?.launcher?.dir ?? await appConfigDir();
            const javaPath = appStore?.config?.java?.java_path || "java"

            switch (instance.type) {
                case "vanilla":
                    return new VanillaInstaller(instance, launcherDir, javaPath)
                case "fabric":
                    return new FabricInstaller(instance, launcherDir, javaPath)
                case "forge":
                    return new ForgeInstaller(instance, launcherDir, javaPath)
                default:
                    throw new LauncherError("UNKNOWN", {
                        message: `Неизвестный тип сборки: ${instance.type}`,
                        context: {instanceId: instance.id, instanceName: instance.name}
                    })
            }
        },

        async runInstance(id: string) {
            const instance = this.getInstance(id)
            if (!instance) {
                captureError(new LauncherError("UNKNOWN", {message: `Сборка ${id} не найдена`}))
                return
            }

            if (this.runningClients.some(c => c.instance.id === id)) return

            const context = {instanceId: instance.id, instanceName: instance.name}

            try {
                const appStore = useAppStore()
                const accountStore = useAccountStore()

                const accounts = accountStore.accountConfig?.accounts ?? []
                const account = accounts[accountStore.accountConfig?.selected ?? 0]
                if (!account) {
                    throw new LauncherError("NO_ACCOUNT", {context})
                }

                // Re-login
                if (account.type == 'microsoft' && Math.floor(Date.now() / 1000) > (account?.expiresAt ?? 0)) {
                    await accountStore.refreshMicrosoftAccount(account.uuid!)
                }

                const client = await this.createInstanceClient(instance)
                await client.prepare()

                const unsubscribe = client.onEvent(e => {
                    if (e.type != 'exit') return

                    this.runningClients = this.runningClients.filter(c => c.id != client.id)
                    unsubscribe()

                    if (e.code !== 0) {
                        captureError(client.crashError(e.code ?? null))
                    }
                })

                await client.run(appStore.config?.java?.java_path || "java", account)
                this.runningClients.push(client)
            } catch (e) {
                captureError(e, {context})
            }
        },

        async createInstanceClient(instance: LivingInstance) {
            const appStore = useAppStore()
            const launcherDir = appStore?.config?.launcher?.dir ?? await appConfigDir();
            switch (instance.type) {
                case "vanilla":
                    return new VanillaClient(launcherDir, instance)
                case "fabric":
                    return new FabricClient(launcherDir, instance)
                case "forge":
                    return new ForgeClient(launcherDir, instance)
                default:
                    throw new LauncherError("UNKNOWN", {
                        message: `Неизвестный тип сборки: ${instance.type}`,
                        context: {instanceId: instance.id, instanceName: instance.name}
                    })
            }
        }
    }
})
