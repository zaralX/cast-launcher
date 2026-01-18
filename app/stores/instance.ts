import {defineStore} from 'pinia'
import type {Instance} from '~/types/instance'
import {appConfigDir} from "@tauri-apps/api/path";
import {path} from "@tauri-apps/api";
import {exists, mkdir} from "@tauri-apps/plugin-fs";

export const useInstanceStore = defineStore('instance', {
    state: () => ({
        instances: [] as Instance[],
        instancesDir: null as null | string,
    }),
    getters: {

    },
    actions: {
        async initInstances() {
            const dataDir = await appConfigDir();
            const instancesDir = await path.join(dataDir, "instances")
            if (!(await exists(instancesDir))) {
                await mkdir(instancesDir, { recursive: true });
            }
            this.instancesDir = instancesDir
        },
    }
})
