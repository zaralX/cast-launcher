<script setup lang="ts">
import {invoke} from "@tauri-apps/api/core";
import {ref} from "vue";
import {useLauncher} from "~/composables/useLauncher";

const {settings, javaList, refreshJavaList} = useLauncher()

const save = async () => {
  await invoke("save_settings", { settings: settings.value })
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
<div class="h-full flex gap-2">
  <div class="bg-neutral-800 rounded-lg p-2 shadow-lg w-48 flex flex-col gap-2">
    <NuxtLink to="/settings">
      <button :class="route.path == '/settings' ? 'text-sky-400 bg-sky-300/[.1]' : 'text-neutral-300 bg-white/[.05]'" class="p-1 rounded-lg w-full font-medium cursor-pointer hover:bg-white/[.1] transition-all duration-200 flex justify-center items-center gap-2">
        <i class="pi pi-cog"></i> Основное
      </button>
    </NuxtLink>
    <NuxtLink to="/settings/profiles">
      <button :class="route.path == '/settings/profiles' ? 'text-sky-400 bg-sky-300/[.1]' : 'text-neutral-300 bg-white/[.05]'" class="p-1 rounded-lg w-full font-medium cursor-pointer hover:bg-white/[.1] transition-all duration-200 flex justify-center items-center gap-2">
        <i class="pi pi-user"></i> Профили
      </button>
    </NuxtLink>
  </div>
  <el-scrollbar height="100%">
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
  </el-scrollbar>
</div>
</template>

<style scoped>

</style>