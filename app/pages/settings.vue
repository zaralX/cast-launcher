<script setup lang="ts">
import {useAppStore} from "~/stores/app";
import {storeToRefs} from "pinia";

definePageMeta({
  layout: "main"
});

const store = useAppStore();
const {config} = storeToRefs(store);

const toast = useToast()
const saving = ref(false)

async function saveConfig() {
  if (!config.value || saving.value) return;
  saving.value = true
  try {
    await store.updateConfig(config.value)
    toast.add({
      title: 'Настройки сохранены',
      color: 'success',
      icon: 'i-lucide-save'
    })
  } catch (e) {
    toast.add({
      title: 'Произошла ошибка',
      description: 'Не получилось сохранить настройки',
      color: 'error',
      icon: 'i-lucide-save'
    })
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="min-h-full w-full px-8 pb-16 pt-10 xl:px-14">
    <!-- Асимметрия: узкая «панель управления» слева, содержательные блоки справа -->
    <div class="grid gap-10 lg:grid-cols-[15rem_minmax(0,1fr)] lg:gap-14">
      <aside class="lg:sticky lg:top-0 lg:self-start">
        <p class="font-mono text-[10px] uppercase tracking-[0.4em] text-fg-faint">Конфигурация</p>
        <h1 class="mt-4 font-unbounded text-[clamp(26px,3vw,34px)] font-bold leading-[0.95] tracking-[-0.055em] text-fg">
          Система<span class="text-acid">.</span>
        </h1>
        <p class="mt-5 text-[12px] leading-relaxed text-fg-muted">
          Изменения применяются после сохранения и хранятся в config.json.
        </p>

        <button
            type="button"
            :disabled="!config || saving"
            class="group/act relative mt-8 flex h-11 w-full items-center justify-center overflow-hidden border border-line font-mono text-[11px] uppercase tracking-[0.2em] text-fg transition-colors duration-300 hover:border-acid hover:text-on-acid disabled:pointer-events-none disabled:opacity-30"
            @click="saveConfig"
        >
          <span
              class="absolute inset-0 origin-left scale-x-0 bg-acid transition-transform duration-500 ease-deck group-hover/act:scale-x-100"
              aria-hidden="true"
          />
          <span class="relative flex items-center gap-2">
            <UIcon
                :name="saving ? 'i-lucide-loader-circle' : 'i-lucide-save'"
                class="size-3.5"
                :class="saving ? 'animate-spin' : ''"
            />
            {{ saving ? 'Сохранение' : 'Сохранить' }}
          </span>
        </button>
      </aside>

      <div v-if="config" class="space-y-6">
        <SettingsLauncher v-model="config" class="animate-rise"/>
        <SettingsAccounts class="animate-rise [animation-delay:80ms]"/>
        <SettingsJava v-model="config" class="animate-rise [animation-delay:160ms]"/>
      </div>

      <div v-else class="flex items-center gap-3 py-14">
        <span class="size-1.5 bg-fg-faint animate-blink"/>
        <p class="font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">Конфигурация не загружена</p>
      </div>
    </div>
  </div>
</template>
