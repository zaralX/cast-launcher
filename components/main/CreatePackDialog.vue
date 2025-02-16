<script setup lang="ts">
import {ref} from "vue";
import {invoke} from "@tauri-apps/api/core";
import axios from "axios";

const model = defineModel()

const newPack = ref({
  packId: "",
  version: "1.12.2",
  versionType: "vanilla"
});
const snapshots = ref(false)
const old = ref(false)
const manifest = ref({})
const selectOptions = ref([])

const createPack = async () => {
  if (newPack.value.packId.length == 0) return;
  model.value = false;
  await invoke("create_pack", newPack.value);
}

onMounted(async () => {
  const response = await axios.get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
  manifest.value = response.data
  newPack.value.version = manifest.value.latest.release
  updateOptions()
})

const updateOptions = () => {
  selectOptions.value = []
  for (const version of manifest.value.versions) {
    if (version.type === "snapshot" && !snapshots.value) continue;
    if (["old_beta", "old_alpha"].includes(version.type) && !old.value) continue;
    selectOptions.value.push({type: version.type, id: version.id})
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
      <p>Pack id</p>
      <el-input v-model="newPack.packId" placeholder="Pack id"
                :formatter='(value) => value.replace(/[<>:\"/\\|?*]+/g, "")'
                :parser='(value) => value.replace(/[<>:\"/\\|?*]+/g, "")'
      />
      <p class="mt-2">Minecraft version</p>
      <!--      <el-input v-model="newPack.version" placeholder="Minecraft version"/>-->
      <el-select v-model="newPack.version" placeholder="Minecraft version">
        <el-option
            v-for="version in selectOptions"
            :key="version.id"
            :label="version.id"
            :value="version.id"
        />
      </el-select>
      <div>
        <el-checkbox v-model="snapshots" @change="updateOptions()" label="Снапшоты"/>
        <el-checkbox v-model="old" @change="updateOptions()" label="Старые версии"/>
      </div>
      <p class="mt-2">Version type</p>
      <el-select v-model="newPack.versionType" placeholder="Version type">
        <el-option
            key="vanilla"
            label="Vanilla"
            value="vanilla"
        />
      </el-select>
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="createPackDialog = false">Отмена</el-button>
          <el-button type="primary" @click="createPack(); createPackDialog = false">
            Создать
          </el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>

</style>