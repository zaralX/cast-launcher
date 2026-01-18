import {defineStore} from 'pinia'
import type {Instance, LivingInstance} from '~/types/instance'
import {appConfigDir} from "@tauri-apps/api/path";
import {path} from "@tauri-apps/api";
import {create, exists, mkdir, readDir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import { v4 as uuidv4 } from "uuid";
import {InstallerBase} from "~/lib/installers/InstallerBase";
import {VanillaInstaller} from "~/lib/installers/VanillaInstaller";

export const useInstanceStore = defineStore('instance', {
    state: () => ({
        instances: [] as LivingInstance[],
        instancesDir: "",
        currentInstaller: null as null | InstallerBase
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
                    dir: await path.join(instancesDir, instanceEntry.name)
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
            // const instanceFile = await create(instanceFileDir)
            await writeTextFile(instanceFileDir, JSON.stringify(data))
        },
        async installInstance(id: string) {
            const instance = this.getInstance(id)
            if (!instance) return;

            const installer = await this.createInstaller(instance)

            this.currentInstaller = installer

            const unsubscribeInstaller = installer.onProgress(p => {
                console.log(p)
                if (p.stage == "finished" || p.stage == "aborted") {
                    this.currentInstaller = null
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
                default:
                    throw new Error("UNKNOWN_INSTALLER")
            }
        },
    }
})
