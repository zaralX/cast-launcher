import {defineStore} from 'pinia'
import type {Instance, LivingInstance} from '~/types/instance'
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

export const useInstanceStore = defineStore('instance', {
    state: () => ({
        instances: [] as LivingInstance[],
        instancesDir: "",
        currentInstaller: null as null | InstallerBase,
        runningClients: [] as ClientBase[]
    }),
    getters: {
        getInstance: (state) => {
            return (id: string) => state.instances.find((instance) => instance.id == id)
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
            let installer: InstallerBase | null = null

            try {
                installer = await this.createInstaller(instance)
                this.currentInstaller = installer
                await installer.install()
            } catch (e) {
                captureError(e, {context: {instanceId: instance.id, instanceName: instance.name}})
            } finally {
                instance.installing = false
                if (this.currentInstaller === installer) this.currentInstaller = null
            }
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
