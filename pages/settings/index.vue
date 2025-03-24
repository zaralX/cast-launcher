<script setup lang="ts">
import {invoke} from "@tauri-apps/api/core";
import {ref} from "vue";
import {useLauncher} from "~/composables/useLauncher";

const {settings, javaList, refreshJavaList} = useLauncher()

const save = async () => {
  await invoke("update_settings", {data: settings.value})
}

onMounted(async () => {
  if (javaList.value.length == 0) {
    await refreshJavaList();
  }
})

const newNickname = ref("")
const route = useRoute()
</script>

<template>
  <div class="h-full">
    <el-scrollbar height="100%">
      <div class="flex flex-col gap-2">
        {{ settings }}
        <p>packs_dir</p>
        <el-input
            v-model="settings.packs_dir"
            style="width: 240px"
            placeholder="Packs dir"
            :parser="(value) => value.replace(/\$\s?|(,*)/g, '')"
        />
        <p>java path</p>
        <div class="flex">
<!--          <el-input-->
<!--              v-model="settings.java_options.path"-->
<!--              style="width: 240px"-->
<!--              placeholder="Java.exe path"-->
<!--              :parser="(value) => value.replace(/\$\s?|(,*)/g, '')"-->
<!--          />-->
<!--          <el-select v-model="settings.java_options.path" placeholder="Java.exe path">-->
<!--            <el-option-->
<!--                v-for="java in javaList"-->
<!--                :key="java?.path"-->
<!--                :label="java?.version + ' - ' + java?.path"-->
<!--                :value="java?.path"-->
<!--            />-->
<!--          </el-select>-->
        </div>

        <el-button @click="save">SAVE</el-button>
      </div>
    </el-scrollbar>
  </div>
</template>

<style scoped>

</style>