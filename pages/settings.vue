<script setup lang="ts">
import {invoke} from "@tauri-apps/api/core";
import {ref} from "vue";

const settings = ref({ java_options: {}, profiles: []})

onMounted(async () => {
  settings.value = await invoke("load_settings", {})
  const java_paths = await invoke("get_java_list", { });
  for (const javaPath of java_paths) {
    const version = await invoke("get_java_version", { javaPath: javaPath });
    javaList.value.push({
      path: javaPath,
      version: version,
    })
  }
})

const save = async () => {
  await invoke("save_settings", { settings: settings.value })
}

const javaList = ref([]);
const newNickname = ref("")
</script>

<template>
<div class="flex flex-col gap-2">
  {{settings}}
  <p>packs_dir</p>
  <el-input
      v-model="settings.packs_dir"
      style="width: 240px"
      placeholder="Packs dir"
      :parser="(value) => value.replace(/\$\s?|(,*)/g, '')"
  />
  <p>java path</p>
  <div class="flex">
    <el-input
        v-model="settings.java_options.path"
        style="width: 240px"
        placeholder="Java.exe path"
        :parser="(value) => value.replace(/\$\s?|(,*)/g, '')"
    />
    <el-select v-model="settings.java_options.path" placeholder="Java.exe path">
      <el-option
          v-for="java in javaList"
          :key="java?.path"
          :label="java?.version + ' - ' + java?.path"
          :value="java?.path"
      />
    </el-select>
  </div>
  <p>profiles</p>
  <div>
    <div class="grid grid-cols-2">
      <p>Никнейм</p>
      <p>Действия</p>
    </div>
    <div class="grid grid-cols-2" v-for="profile in settings.profiles">
      <p>{{profile.username}}</p>
      <div>
        <el-button type="danger" size="small" @click="settings.profiles = settings.profiles.filter((p) => p.username != profile.username)">Удалить</el-button>
        <el-button type="success" :disabled="profile.selected" size="small" @click="settings.profiles = settings.profiles.filter((p) => {p.selected = p.username == profile.username; return true})">Выбрать</el-button>
      </div>
    </div>
  </div>
  <div class="flex">
    <el-input placeholder="Никнейм" v-model="newNickname" />
    <el-button @click="settings.profiles.push({username: newNickname, selected: false}); newNickname = ''">Добавить профиль</el-button>
  </div>

  <el-button @click="save">SAVE</el-button>
</div>
</template>

<style scoped>

</style>