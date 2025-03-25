<script setup>
import {ref} from "vue";
import {invoke} from "@tauri-apps/api/core";
import axios from "axios";
import {useLauncher} from "~/composables/useLauncher";
import { parseStringPromise } from "xml2js";

const model = defineModel()
const {settings, javaList} = useLauncher()

const newPack = ref({
  packName: "",
  version: "1.12.2",
  versionType: "vanilla",
  javaPath: null
});
const snapshots = ref(false)
const old = ref(false)
const manifest = ref({})
const selectOptions = ref([])
const javaRequired = ref(8)

const createPack = async () => {
  if (newPack.value.packId.length == 0) return;
  model.value = false;
  const sendData = {
    name: newPack.value.packName,
    type: newPack.value.versionType,
    version: newPack.value.version
  };
  if (sendData.type === "forge") {
    sendData["forge-version"] = forgeVersion.value;
  } else if (sendData.type === "fabric") {
    sendData["fabric-loader"] = fabricVersion.value;
  }

  if (newPack.value.javaPath) {
    sendData.java = newPack.value.javaPath;
  }
  await invoke("create_pack", {data: sendData});
}

onMounted(async () => {
  const response = await axios.get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
  manifest.value = response.data
  newPack.value.version = manifest.value.latest.release
  updateOptions()
  // newPack.value.javaPath = settings.value.java_options.path
  await updateRequiredJava()

  const res = await axios.get("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml");
  const data = await parseStringPromise(res.data);
  forgeVersionsALL.value = data.metadata.versioning[0].versions[0].version.map(value => {
    const split = value.split("-");
    return { minecraft: split[0], forge: split[1] };
  });
})

const updateOptions = () => {
  selectOptions.value = []
  for (const version of manifest.value.versions) {
    if (version.type === "snapshot" && !snapshots.value) continue;
    if (["old_beta", "old_alpha"].includes(version.type) && !old.value) continue;
    selectOptions.value.push({type: version.type, id: version.id})
  }
}

const updateRequiredJava = async () => {
  const versionData = manifest.value.versions.find(version => version.id === newPack.value.version)
  const response = await axios.get(versionData.url)
  const data = response.data;
  console.log(data)
  javaRequired.value = data.javaVersion.majorVersion
}

const forgeVersion = ref("latest")
const fabricVersion = ref("latest")

const forgeVersionsALL = ref([])
const forgeVersions = ref([])
const fabricVersions = ref([])

const versionTypeUpdate = async (newValue) => {
  if (newValue === 'forge') {
    forgeVersions.value = forgeVersionsALL.value.filter(value => {
      return value.minecraft === newPack.value.version
    });
  } else if (newValue === 'fabric') {
    fabricVersions.value = (await axios.get("https://meta.fabricmc.net/v2/versions/loader/" + newPack.value.version))
        .data.map(v => {
          return v.loader.version
        })
  }
}
</script>

<template>
  <div>
    <el-dialog
        v-model="model"
        title="Создание сборки"
        width="400"
    >
      <p>Pack name</p>
      <el-input v-model="newPack.packName" placeholder="Pack name"
                :formatter='(value) => value.replace(/[<>:\"/\\|?*]+/g, "")'
                :parser='(value) => value.replace(/[<>:\"/\\|?*]+/g, "")'
      />
      <p class="mt-2">Minecraft version</p>
      <!--      <el-input v-model="newPack.version" placeholder="Minecraft version"/>-->
      <el-select v-model="newPack.version" placeholder="Minecraft version" @change="updateRequiredJava()">
        <el-option
            v-for="version in selectOptions"
            :key="version.id"
            :label="version.id"
            :value="version.id"
        />
      </el-select>
      <div>
        <el-checkbox v-model="snapshots" @change="updateOptions()" label="Снапшоты"/>
        <el-tooltip content="Не поддерживается :(" effect="light" placement="right">
          <el-checkbox disabled v-model="old" @change="updateOptions()" label="Старые версии"/>
        </el-tooltip>
      </div>
      <p class="mt-2">Version type</p>
      <el-select v-model="newPack.versionType" @change="versionTypeUpdate" placeholder="Version type">
        <el-option
            key="vanilla"
            label="Vanilla"
            value="vanilla"
        />
        <el-option
            key="forge"
            label="Forge"
            value="forge"
        />
        <el-option
            key="fabric"
            label="Fabric"
            value="fabric"
        />
      </el-select>
      <div v-if="newPack.versionType === 'forge'">
        <p class="mt-2">Forge version</p>
        <el-select v-model="forgeVersion" placeholder="Version type">
          <el-option
              key="latest"
              label="Последняя версия"
              value="latest"
          />
          <el-option
              v-for="version in forgeVersions"
              :key="version.forge"
              :label="version.forge"
              :value="version.forge"
          />
        </el-select>
      </div>
      <div v-if="newPack.versionType === 'fabric'">
        <p class="mt-2">Fabric version</p>
        <el-select v-model="fabricVersion" placeholder="Version type">
          <el-option
              key="latest"
              label="Последняя версия"
              value="latest"
          />
          <el-option
              v-for="version in fabricVersions"
              :key="version"
              :label="version"
              :value="version"
          />
        </el-select>
      </div>
      <p class="mt-2">Java</p>
      <p v-if="javaRequired != javaList.find(java => java.path == newPack.javaPath)?.version" class="text-xs text-yellow-500"><i class="pi pi-exclamation-triangle"></i> Для версии {{newPack.version}} требуется Java {{javaRequired}}</p>
      <el-input v-model="newPack.javaPath" />
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="model = false">Отмена</el-button>
          <el-button type="primary" @click="createPack(); model = false">
            Создать
          </el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>

</style>