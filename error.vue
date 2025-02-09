<script setup lang="ts">
import type { NuxtError } from '#app'

defineProps({
  error: Object as () => NuxtError
})
</script>

<template>
  <div class="h-screen w-full flex justify-center items-center">
    <div>
      <el-result
          icon="error"
      >
        <template #sub-title>
          <p class="!text-lg">Внимание, ошибка!</p>
        </template>
        <template #title>
          <p class="!text-2xl">{{error?.statusCode}}</p>
        </template>
        <template #extra>
          <NuxtLink to="/"><el-button type="primary">Вернуться назад</el-button></NuxtLink>
        </template>
      </el-result>
    </div>
  </div>
</template>