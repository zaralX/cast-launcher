<script setup lang="ts">
import {useLauncher} from "~/composables/useLauncher";
import {invoke} from "@tauri-apps/api/core";

const {settings} = useLauncher()

const select = async (username: string) => {
  settings.value.profiles = settings.value.profiles.filter((p) => {p.selected = p.username === username; return true})
  await invoke("save_settings", { settings: settings.value })
  ElMessage({
    message: 'Изменения сохранены',
    type: 'success',
    plain: true,
  })
}

const deleteProfile = async (username: string) => {
  let found = false
  if (settings.value.profiles.find(p => p.username === username).selected) {
    found = true;
  }
  settings.value.profiles = settings.value.profiles.filter((p) => p.username != username)
  if (settings.value.profiles.length > 0 && found) {
    settings.value.profiles[0].selected = true;
  }
  await invoke("save_settings", { settings: settings.value })
  ElMessage({
    message: 'Изменения сохранены',
    type: 'success',
    plain: true,
  })
}

const addOfflineDialog = ref(false)
const offlineUsername = ref("")

const addOffline = async () => {
  if (offlineUsername.value.length == 0) return;
  addOfflineDialog.value = false
  settings.value.profiles.push({username: offlineUsername.value, selected: false});
  await select(offlineUsername.value);
  offlineUsername.value = "";
}

const addOfflineHandleClose = async (done: () => void) => {
  await addOffline()
  done()
}
</script>

<template>
<div class="h-full w-full flex flex-col gap-2">
  <div class="h-full w-full">
    <el-dialog
        v-model="addOfflineDialog"
        title="Добавить автономную"
        width="400"
        :before-close="addOfflineHandleClose"
    >
      <el-input v-model="offlineUsername" placeholder="Никнейм" maxlength="16" />
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="addOfflineDialog = false">Отмена</el-button>
          <el-button type="primary" @click="addOffline">
            Создать
          </el-button>
        </div>
      </template>
    </el-dialog>
    <el-table :data="settings.profiles" height="100%" width="100%">
      <el-table-column prop="selected" label="Выбран" width="100">
        <template #default="scope">
          <el-checkbox v-model="settings.profiles.find(p => p.username === scope.row.username).selected" @change="() => select(scope.row.username)" />
        </template>
      </el-table-column>
      <el-table-column prop="username" label="Имя пользователя" />
      <el-table-column prop="authorized-name" label="Учётная запись" />
      <el-table-column prop="type" label="Тип" />
      <el-table-column prop="delete" label="Удалить" width="128">
        <template #default="scope">
          <el-button size="small" type="danger" @click="() => deleteProfile(scope.row.username)" plain>
            Удалить
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </div>
  <div class="bg-neutral-800 rounded-lg p-2">
    <el-button type="info" plain @click="addOfflineDialog = true">
      <i class="pi pi-user"></i>  Добавить автономную
    </el-button>
    <el-button type="info" plain disabled>
      <i class="pi pi-microsoft"></i>  Войти через Microsoft
    </el-button>
  </div>
</div>
</template>

<style scoped>

</style>