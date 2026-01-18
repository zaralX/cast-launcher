import {defineStore} from 'pinia'
import type {Instance} from '~/types/instance'
import {appConfigDir} from "@tauri-apps/api/path";
import {path} from "@tauri-apps/api";
import {create, exists, mkdir, writeTextFile} from "@tauri-apps/plugin-fs";
import { v4 as uuidv4 } from "uuid";

export const useInstanceStore = defineStore('instance', {
    state: () => ({
        instances: [] as Instance[],
        instancesDir: "",
        currentlyCreating: null
    }),
    getters: {

    },
    actions: {
        async initInstances() {
            const dataDir = await appConfigDir();
            const instancesDir = await path.join(dataDir, 'instances')
            if (!(await exists(instancesDir))) {
                await mkdir(instancesDir, { recursive: true });
            }
            this.instancesDir = instancesDir
        },
        async createInstance(data: Instance) {
            let instanceDir = await path.join(this.instancesDir, data.id)
            if (await exists(instanceDir)) {
                const randomId = uuidv4().split('-')[0] as string
                instanceDir = await path.join(this.instancesDir, `${data.id}-${randomId}`)
            }
            console.log(2)
            await mkdir(instanceDir, { recursive: true })

            const instanceFileDir = await path.join(instanceDir, "instance.json")
            // const instanceFile = await create(instanceFileDir)
            await writeTextFile(instanceFileDir, JSON.stringify(data))
        }
    }
})
