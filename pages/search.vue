<script setup>
import axios from "axios";
import {invoke} from "@tauri-apps/api/core";

const versions = ref([])

const filterSources = ref([])
const filterVersions = ref([])

const modrinthSearch = ref({})

onMounted(async () => {
  const response = await axios.get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
  versions.value = []
  for (const version of response.data.versions) {
    if (version.type === "snapshot") continue;
    if (["old_beta", "old_alpha"].includes(version.type)) continue;
    versions.value.push(version.id)
  }
  await getPacks();
})

const getPacks = async () => {
  const versionsString = ",[" + filterVersions.value.map(v => "\"versions:" + v + "\"").join(',') + "]"
  const url = "https://api.modrinth.com/v2/search?facets=[[\"project_type:modpack\"]"+(versionsString === ',[]' ? '' : versionsString)+"]";
  console.log(url);
  modrinthSearch.value = (await axios.get(url)).data
}

const createAndInstall = async (project) => {
  await invoke("create_pack", {data: {
      id: project.slug,
      name: project.title,
      type: "modrinth",
      "modrinth-project-id": project.project_id,
      "modrinth-project-version": project.latest_version,
    }});
}
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex mt-2 h-full">
      <div class="w-64 h-full p-2 bg-neutral-800 rounded-xl shadow-md">
        <div>
          <p class="font-medium text-lg">Источники</p>
          <el-checkbox-group :min="1" v-model="filterSources">
            <el-checkbox checked label="Modrinth" value="modrinth" />
<!--            need add forge support and make it TODO-->
            <el-checkbox disabled label="Nazzy projects" value="nazzy" />
          </el-checkbox-group>
        </div>
        <div>
          <p class="font-medium text-lg">Версии</p>
          <el-select
              v-model="filterVersions"
              multiple
              collapse-tags
              collapse-tags-tooltip
              placeholder="Select"
              style="width: 240px"
              @change="getPacks"
          >
            <el-option
                v-for="item in versions"
                :key="item"
                :label="item"
                :value="item"
            />
          </el-select>
        </div>
      </div>
      <el-scrollbar height="100%" class="w-full">
        <div v-for="hit in modrinthSearch?.hits">
          <p>hit.title {{hit.title}}</p>
          <p>hit.description {{hit.title}}</p>
          <p>hit.project_id {{hit.project_id}}</p>
          <el-button @click="createAndInstall(hit)">Install</el-button>
        </div>
      </el-scrollbar>
    </div>
  </div>
</template>

<style scoped>
.el-segmented {
  --el-border-radius-base: 16px;
}
</style>