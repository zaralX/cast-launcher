import {defineStore} from 'pinia'
import type {Instance, LivingInstance} from '~/types/instance'
import {appConfigDir} from "@tauri-apps/api/path";
import {path} from "@tauri-apps/api";
import {create, exists, mkdir, readDir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import { v4 as uuidv4 } from "uuid";
import {InstallerBase} from "~/lib/installers/InstallerBase";
import {VanillaInstaller} from "~/lib/installers/VanillaInstaller";
import {ClientBase} from "~/lib/client/ClientBase";
import { VanillaClient } from "~/lib/client/VanillaClient";
import {FabricInstaller} from "~/lib/installers/FabricInstaller";
import {FabricClient} from "~/lib/client/FabricClient";
import { ForgeInstaller } from "~/lib/installers/ForgeInstaller";
import {ForgeClient} from "~/lib/client/ForgeClient";

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
            if (!(await exists(instancesDir))) {
                await mkdir(instancesDir, { recursive: true });
            }
            this.instancesDir = instancesDir

            // Initializing all instances from /instances dir to this.instances
            this.instances = []
            const instanceEntries = await readDir(instancesDir);
            for (const instanceEntry of instanceEntries) {
                if (!instanceEntry.isDirectory) continue

                const instanceFileDir = await path.join(instancesDir, instanceEntry.name, "instance.json")
                if (!(await exists(instanceFileDir))) continue

                const instanceFileContent = await readTextFile(instanceFileDir)
                const instanceConfig = JSON.parse(instanceFileContent) as Instance
                this.instances.push({
                    ...instanceConfig,
                    dir: await path.join(instancesDir, instanceEntry.name),
                    installing: false
                })
            }
        },
        async createInstance(data: Instance) {
            let instanceDir = await path.join(this.instancesDir, data.id)
            if (await exists(instanceDir)) {
                const randomId = uuidv4().split('-')[0] as string
                instanceDir = await path.join(this.instancesDir, `${data.id}-${randomId}`)
            }
            await mkdir(instanceDir, { recursive: true })

            const instanceFileDir = await path.join(instanceDir, "instance.json")
            await writeTextFile(instanceFileDir, JSON.stringify(data))

            await this.initInstances()
        },
        async installInstance(id: string) {
            const instance = this.getInstance(id)
            if (!instance) return;

            instance.installing = true

            const installer = await this.createInstaller(instance)

            this.currentInstaller = installer

            const unsubscribeInstaller = installer.onProgress(p => {
                console.log(p)
                if (p.stage == "finished" || p.stage == "aborted") {
                    this.currentInstaller = null
                    instance.installing = false
                    unsubscribeInstaller()
                }
            })

            await installer.install()
        },

        async createInstaller(instance: LivingInstance) {
            const appStore = useAppStore()
            const launcherDir = appStore?.config?.launcher?.dir ?? await appConfigDir();
            switch (instance.type) {
                case "vanilla":
                    return new VanillaInstaller(instance, launcherDir)
                case "fabric":
                    return new FabricInstaller(instance, launcherDir)
                case "forge":
                    return new ForgeInstaller(instance, launcherDir)
                default:
                    throw new Error("UNKNOWN_INSTALLER")
            }
        },

        async runInstance(id: string) {
            const instance = this.getInstance(id)
            if (!instance) return;

            const client = await this.createInstanceClient(instance)

            await client.prepare()
            const unsubscribe = client.onEvent(e => {
                if (e.type == 'exit') {
                    this.runningClients = this.runningClients.filter(c => {
                        console.log(c.id != client.id)
                        return c.id != client.id
                    })
                    unsubscribe()
                }
            })

            const { config } = useAppStore()
            const { accountConfig } = useAccountStore()
            const account = accountConfig!.accounts[accountConfig!.selected ?? 0]
            if (!account) {
                unsubscribe()
                return;
            }

            // Re-login
            if (account.type == 'microsoft' && Math.floor(Date.now() / 1000) > (account?.expiresAt ?? 0)) {
                await useAccountStore().refreshMicrosoftAccount(account.uuid!)
            }

            await client.run(config!.java.java_path ?? "java", account)
            this.runningClients.push(client)
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
                    throw new Error("UNKNOWN_CLIENT_TYPE")
            }
        }
    }
})
