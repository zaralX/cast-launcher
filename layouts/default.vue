<script setup lang="ts">
const route = useRoute()
</script>

<template>
  <div class="flex flex-col">
    <MainAppbar />
    <div class="flex-1 flex bg-neutral-800">
      <div class="w-16 bg-neutral-800 flex flex-col items-center py-2">
        <div class="flex flex-col items-center py-2 gap-2 flex-1">
          <el-tooltip effect="light" placement="right" content="Главная">
            <NuxtLink to="/">
              <button :class="route.path === '/' ? 'text-blue-400 bg-sky-300/[.1]' : 'text-neutral-200'" class="cursor-pointer w-10 h-10 flex justify-center items-center rounded-lg text-lg hover:shadow-lg hover:bg-neutral-700 transition duration-200">
                <div :class="route.path === '/' ? 'h-6' : 'h-0'" class="w-1 bg-blue-400 absolute left-0 rounded-r-md transition-all duration-200 -translate-x-0.5"></div>
                <div :class="route.path === '/' ? 'h-8' : 'h-0'" class="w-4 bg-blue-400 absolute left-0 rounded-r-md transition-all duration-200 -translate-x-2 blur-md opacity-50"></div>
                <i class="pi pi-home"></i>
              </button>
            </NuxtLink>
          </el-tooltip>
          <el-tooltip effect="light" placement="right" content="Тест">
            <NuxtLink to="/test">
              <button :class="route.path === '/test' ? 'text-blue-400 bg-sky-300/[.1]' : 'text-neutral-200'" class="cursor-pointer w-10 h-10 flex justify-center items-center rounded-lg text-lg hover:shadow-lg hover:bg-neutral-700 transition duration-200">
                <div :class="route.path === '/test' ? 'h-6' : 'h-0'" class="w-1 bg-blue-400 absolute left-0 rounded-r-md transition-all duration-200 -translate-x-0.5"></div>
                <div :class="route.path === '/test' ? 'h-8' : 'h-0'" class="w-4 bg-blue-400 absolute left-0 rounded-r-md transition-all duration-200 -translate-x-2 blur-md opacity-50"></div>
                <i class="pi pi-clipboard"></i>
              </button>
            </NuxtLink>
          </el-tooltip>
        </div>
        <div class="flex flex-col items-center py-2 gap-2">
          <el-tooltip effect="light" placement="right" content="Настройки">
            <NuxtLink to="/settings">
              <button :class="route.path === '/settings' ? 'text-blue-400 bg-sky-300/[.1]' : 'text-neutral-200'" class="cursor-pointer w-10 h-10 flex justify-center items-center rounded-lg text-lg hover:shadow-lg hover:bg-neutral-700 transition duration-200">
                <div :class="route.path === '/settings' ? 'h-6' : 'h-0'" class="w-1 bg-blue-400 absolute left-0 rounded-r-md transition-all duration-200 -translate-x-0.5"></div>
                <div :class="route.path === '/' ? 'h-8' : 'h-0'" class="w-4 bg-blue-400 absolute left-0 rounded-r-md transition-all duration-200 -translate-x-2 blur-md opacity-50"></div>
                <i class="pi pi-cog"></i>
              </button>
            </NuxtLink>
          </el-tooltip>
        </div>
      </div>
      <div class="flex-1 h-[calc(100vh-3rem)] bg-neutral-900 rounded-tl-2xl p-3 shadow-[4px_4px_8px_0px_rgba(0,0,0,0.3)_inset]">
        <slot/>
      </div>
    </div>
  </div>
</template>

<style scoped>

</style>