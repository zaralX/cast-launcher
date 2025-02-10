import { ref } from 'vue';
import {invoke} from "@tauri-apps/api/core";

const settings = ref({ java_options: {}, profiles: [] });
const packs = ref([])
const javaList = ref([])

export const useLauncher = () => {

    async function initData() {
        settings.value = await invoke("load_settings", {});
        await refreshPacks();
    }

    async function refreshPacks() {
        packs.value = await invoke("get_packs", { launcherDir: settings.value?.packs_dir });
    }

    async function refreshJavaList() {
        javaList.value = []
        const java_paths = await invoke("get_java_list", { });
        for (const javaPath of java_paths) {
            const version = await invoke("get_java_version", { javaPath: javaPath });
            javaList.value.push({
                path: javaPath,
                version: version,
            })
        }
    }

    return {
        settings,
        initData,
        packs,
        refreshPacks,
        javaList,
        refreshJavaList,
    };
};
