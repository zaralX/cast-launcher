<script setup>

import ModrinthSearch from "~/components/search/ModrinthSearch.vue";

const selectedModPlace = ref("modrinth")

const modPlaces = [
  {
    label: "Modrinth",
    value: "modrinth",
    icon: "/modrinth.svg",
  },
  {
    label: "Curseforge",
    value: "curseforge",
    icon: "/curseforge.svg",
    disabled: true,
  }
]
</script>

<template>
  <div class="h-full flex flex-col">
    <el-segmented v-model="selectedModPlace" :options="modPlaces" block>
      <template #default="scope">
        <div class="flex items-center justify-center gap-2 p-2">
          <img :src="scope.item.icon" alt="icon" class="w-4 h-4" />
          <div class="">{{ scope.item.label }}</div>
        </div>
      </template>
    </el-segmented>
    <ModrinthSearch />
  </div>
</template>

<style scoped>
.el-segmented {
  --el-border-radius-base: 16px;
}
</style>