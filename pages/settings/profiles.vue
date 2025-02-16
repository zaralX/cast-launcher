<script setup lang="ts">
import {useLauncher} from "~/composables/useLauncher";
import {invoke} from "@tauri-apps/api/core";

const {settings} = useLauncher()

const select = async (username: string) => {
  settings.value.profiles = settings.value.profiles.filter((p) => {
    p.selected = p.username === username;
    return true
  })
  await invoke("save_settings", {settings: settings.value})
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
  await invoke("save_settings", {settings: settings.value})
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

const addOnline = async () => {
  const CLIENT_ID = "ВАШ_CLIENT_ID";
  const REDIRECT_URI = "https://login.microsoftonline.com/common/oauth2/nativeclient";

  // Открываем окно авторизации Microsoft
  const authUrl = `https://login.live.com/oauth20_authorize.srf?client_id=${CLIENT_ID}&response_type=code&redirect_uri=${REDIRECT_URI}&scope=XboxLive.signin%20offline_access`;

  window.open(authUrl, "_blank");

  // Ожидаем код из редиректа (потребуется слушатель или промежуточный сервер)
  const code = await getAuthCodeFromRedirect(); // Реализуйте этот метод

  // Получаем access_token от Microsoft
  const tokenResponse = await fetch("https://login.live.com/oauth20_token.srf", {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      client_id: CLIENT_ID,
      code,
      grant_type: "authorization_code",
      redirect_uri: REDIRECT_URI,
      client_secret: "ВАШ_CLIENT_SECRET",
    }),
  });

  const tokenData = await tokenResponse.json();
  const microsoftAccessToken = tokenData.access_token;

  // Получаем Xbox токен
  const xboxResponse = await fetch("https://user.auth.xboxlive.com/user/authenticate", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      Properties: {
        AuthMethod: "RPS",
        SiteName: "user.auth.xboxlive.com",
        RpsTicket: `d=${microsoftAccessToken}`,
      },
      RelyingParty: "http://auth.xboxlive.com",
      TokenType: "JWT",
    }),
  });

  const xboxData = await xboxResponse.json();
  const xboxToken = xboxData.Token;
  const userHash = xboxData.DisplayClaims.xui[0].uhs;

  // Получаем XSTS токен
  const xstsResponse = await fetch("https://xsts.auth.xboxlive.com/xsts/authorize", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      Properties: {
        SandboxId: "RETAIL",
        UserTokens: [xboxToken],
      },
      RelyingParty: "rp://api.minecraftservices.com/",
      TokenType: "JWT",
    }),
  });

  const xstsData = await xstsResponse.json();
  const xstsToken = xstsData.Token;

  // Получаем Minecraft токен
  const mcResponse = await fetch("https://api.minecraftservices.com/authentication/login_with_xbox", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      identityToken: `XBL3.0 x=${userHash};${xstsToken}`,
    }),
  });

  const mcData = await mcResponse.json();
  const minecraftAccessToken = mcData.access_token;

  // Получаем профиль игрока
  const profileResponse = await fetch("https://api.minecraftservices.com/minecraft/profile", {
    method: "GET",
    headers: {
      Authorization: `Bearer ${minecraftAccessToken}`,
    },
  });

  const profileData = await profileResponse.json();

  // Сохраняем данные в localStorage или Vuex/Pinia
  localStorage.setItem("mc_access_token", minecraftAccessToken);
  localStorage.setItem("mc_uuid", profileData.id);
  localStorage.setItem("mc_username", profileData.name);

  console.log("Вход выполнен:", profileData.name);
};


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
        <el-input v-model="offlineUsername" placeholder="Никнейм" maxlength="16"/>
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
            <el-checkbox v-model="settings.profiles.find(p => p.username === scope.row.username).selected"
                         @change="() => select(scope.row.username)"/>
          </template>
        </el-table-column>
        <el-table-column prop="username" label="Имя пользователя"/>
        <el-table-column prop="authorized-name" label="Учётная запись"/>
        <el-table-column prop="type" label="Тип"/>
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
      <el-button type="info" plain @click="addOnline" disabled>
        <i class="pi pi-microsoft"></i>  Войти через Microsoft
      </el-button>
    </div>
  </div>
</template>

<style scoped>

</style>