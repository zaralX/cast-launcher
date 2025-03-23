import { ref } from 'vue';
import {invoke} from "@tauri-apps/api/core";
import {getVersion} from "@tauri-apps/api/app";

const version = ref("Not found");
const settings = ref({});
const packs = ref([])
const javaList = ref([])
const clients = ref([])

export const useLauncher = () => {

    async function initData() {
        settings.value = await invoke("get_settings");
        await refreshPacks();
        version.value = await getVersion();
    }

    async function refreshPacks() {
        packs.value = await invoke("get_packs", { });
    }

    async function refreshJavaList() {
        // javaList.value = []
        // const java_paths = await invoke("get_java_list", { });
        // for (const javaPath of java_paths) {
        //     const version = await invoke("get_java_version", { javaPath: javaPath });
        //     javaList.value.push({
        //         path: javaPath,
        //         version: version,
        //     })
        // }
    }

    function updateClientState(pack_id, state) {
        const client = clients.value.find((c) => c.pack_id === pack_id);
        if (!client) {
            clients.value.push({pack_id, state});
            return;
        }

        client.state = state;
    }

    return {
        settings,
        initData,
        packs,
        refreshPacks,
        javaList,
        refreshJavaList,
        version,
        updateClientState,
        clients,
    };
};
