<script setup>
import {onMounted, onUnmounted, ref} from "vue";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import {getVersion} from "@tauri-apps/api/app";

const colorMode = useColorMode()
colorMode.preference = 'dark';
const isDark = ref(false)

watch(isDark, () => {
  colorMode.preference = isDark.value ? 'dark' : 'light';
})

const greetMsg = ref("");
const name = ref("");
const currentDownloading = ref(null);
const java = ref("C:/Users/Miste/.jdks/graalvm-ce-21.0.2/bin/java.exe");
const launcher_dir = ref("D:/RustProjects/cast-launcher/test");
const appVersion = ref('');
const newVersionData = ref({});
const javaList = ref([]);
let unlisten = null;

async function greet() {
  // await invoke("create_pack", {data: {
  //     id: "zapi",
  //     name: "zAPI test",
  //     type: "zapi",
  //     "zapi-project-id": "2",
  //     "zapi-project-version": "1"
  //   }});

  // await invoke("install_pack", {id: "fabulously-optimized"});

  // await invoke("run_pack", {id: "test3"});

  // await invoke("greet", {name: "1"});

  // console.log(await invoke("get_settings"));
}

onMounted(async () => {
  isDark.value = colorMode.preference === 'dark' || colorMode.preference === 'system'
  appVersion.value = await getVersion();
  unlisten = await listen("launching", (event) => {
    currentDownloading.value = event.payload;
  });
  const java_paths = await invoke("get_java_list", {});
  for (const javaPath of java_paths) {
    const version = await invoke("get_java_version", {javaPath: javaPath});
    javaList.value.push({
      path: javaPath,
      version: version,
    })
  }
})

onUnmounted(() => {
  if (unlisten) unlisten();
})
</script>

<template>
  <main class="container">
    <h1>Welcome to Tauri + Vue</h1>

    <div class="row">
      <a href="https://vitejs.dev" target="_blank">
        <img src="/vite.svg" class="logo vite" alt="Vite logo"/>
      </a>
      <a href="https://tauri.app" target="_blank">
        <img src="/tauri.svg" class="logo tauri" alt="Tauri logo"/>
      </a>
      <a href="https://vuejs.org/" target="_blank">
        <img src="../assets/vue.svg" class="logo vue" alt="Vue logo"/>
      </a>
    </div>
    <p>Click on the Tauri, Vite, and Vue logos to learn more.</p>

    <form class="row" @submit.prevent="greet">
      <input id="greet-input" v-model="name" placeholder="Enter a name..."/>
      <button type="submit">Greet</button>
    </form>
    <p>{{ greetMsg }}</p>
    <p class="bg-red-500">Tailwind test</p>
    <el-button>Element Plus test</el-button>
    <p>color: {{ colorMode.preference }}</p>
    <p class="bg-blue-500">current: {{ currentDownloading }}</p>
    <p>java</p>
    <input v-model="java">
    <p>launcher dir</p>
    <input v-model="launcher_dir">
    <p>Version: {{ appVersion }} <span v-if="newVersionData?.available">[Доступно обновление]</span> <span v-else>[Последняя версия]</span>
    </p>
    Javalist:
    <p v-for="java in javaList">
      <span>{{ java?.version }}</span>
      <span> - </span>
      <span>{{ java?.path }}</span>
    </p>
  </main>
</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}


:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}

button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }

  button:active {
    background-color: #0f0f0f69;
  }
}

</style>