<script setup lang="ts">
import axios from "axios";
import {invoke} from "@tauri-apps/api/core";

const loaders = ref([
  {
    id: "fabric",
    label: "Fabric",
    selected: false
  },
  {
    id: "forge",
    label: "Forge",
    disabled: true,
    selected: false
  }
]);

const versions = ref([])

const categories = ref([
  {
    id: "adventure",
    label: "Adventure",
    selected: false
  },
  {
    id: "challenging",
    label: "Challenging",
    selected: false
  }
]);

const openSource = ref(false);

const searchResult = ref({})

const getPacks = async () => {
  searchResult.value = (await axios.get("https://api.modrinth.com/v2/search?facets=[[\"project_type:modpack\"]]")).data
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

onMounted(async () => {
  await getPacks();
})
</script>

<template>
  <div class="flex mt-2 h-full">
    <div class="w-64 p-2 bg-neutral-800 rounded-xl shadow-md h-auto"></div>
    <el-scrollbar height="100%">
      <div v-for="hit in searchResult?.hits">
        <p>hit.title {{hit.title}}</p>
        <p>hit.description {{hit.title}}</p>
        <p>hit.project_id {{hit.project_id}}</p>
        <el-button @click="createAndInstall(hit)">Install</el-button>
      </div>
    </el-scrollbar>
  </div>
</template>

<style scoped>

</style>