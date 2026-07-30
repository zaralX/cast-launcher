<script setup lang="ts">
import {getVersion, getTauriVersion} from "@tauri-apps/api/app"
import {call} from "~/types/backend"

definePageMeta({
  layout: "main"
})

type Entry = {
  label: string
  value: string
  icon: string
  url?: string
  copy?: string
}

const REPO = "https://github.com/zaralX/cast-launcher"

const contacts: Entry[] = [
  {label: "Сайт", value: "zaralx.ru", icon: "i-lucide-globe", url: "https://zaralx.ru"},
  {label: "GitHub", value: "@zaralX", icon: "i-simple-icons-github", url: "https://github.com/zaralX"},
  {label: "Telegram", value: "@zWork1", icon: "i-simple-icons-telegram", url: "https://t.me/zWork1"},
  {label: "Почта", value: "admin@zaralx.ru", icon: "i-lucide-mail", copy: "admin@zaralx.ru"}
]

const project: Entry[] = [
  {label: "Исходники", value: "zaralX/cast-launcher", icon: "i-lucide-code-xml", url: REPO},
  {label: "Баги и идеи", value: "Issues", icon: "i-lucide-bug", url: `${REPO}/issues`},
  {label: "Версии", value: "Releases", icon: "i-lucide-package", url: `${REPO}/releases`},
  {label: "Лицензия", value: "Apache-2.0", icon: "i-lucide-scale", url: `${REPO}/blob/main/LICENSE`}
]

const stack = [
  "Rust", "Tauri 2", "Nuxt 4", "Vue 3", "TypeScript", "Tailwind CSS", "Nuxt UI", "Pinia"
]

const thanks = [
  {name: "Mojang Studios", note: "Minecraft и метаданные в открытом доступе"},
  {name: "Modrinth", note: "открытое API и референс дизайна лаунчера"},
  {name: "PrismLauncher", note: "образец того, как надо делать лаунчеры"},
  {name: "Все, кто тестирует и репортит баги", note: "w w вы крутышки"}
]

const toast = useToast()

const version = ref("")
const tauriVersion = ref("")

onMounted(async () => {
  try {
    version.value = await getVersion()
  } catch {
    version.value = ""
  }

  try {
    tauriVersion.value = await getTauriVersion()
  } catch {
    tauriVersion.value = ""
  }
})

async function activate(entry: Entry) {
  if (entry.url) {
    await safeRun(() => call("open_url", {url: entry.url!}), {
      context: {action: "Открытие ссылки", url: entry.url}
    })
    return
  }

  if (!entry.copy) return

  const copied = await copyToClipboard(entry.copy)

  toast.add({
    title: copied ? "Скопировано" : "Не получилось скопировать",
    description: entry.copy,
    color: copied ? "success" : "error",
    icon: copied ? "i-lucide-clipboard-check" : "i-lucide-clipboard-x"
  })
}
</script>

<template>
  <div class="relative min-h-full w-full overflow-hidden px-6 pb-16 pt-14 xl:px-10">
    <div
        class="pointer-events-none absolute left-1/2 top-0 -z-0 h-80 w-80 -translate-x-1/2 -translate-y-1/3 rounded-full bg-acid/[0.07] blur-[120px]"
        aria-hidden="true"
    />

    <div class="relative mx-auto flex w-full max-w-3xl flex-col items-center">
      <header class="animate-rise flex flex-col items-center text-center">
        <div class="relative grid size-20 place-items-center border border-line bg-ink-800 cut-16">
          <img src="/logo.png" class="size-11" alt="Cast Launcher"/>
          <span class="absolute bottom-0 left-0 h-px w-full bg-gradient-to-r from-transparent via-acid to-transparent"/>
        </div>

        <p class="mt-7 font-mono text-[10px] uppercase tracking-[0.4em] text-fg-faint">Об авторах</p>

        <h1 class="mt-4 font-unbounded text-[clamp(30px,6vw,52px)] font-bold leading-[0.9] tracking-[-0.06em] text-fg">
          CAST<span class="text-acid">.</span>LAUNCHER
        </h1>

        <p class="mt-5 max-w-md text-[12px] leading-relaxed text-fg-muted">
          Очень крутой важный текст без которого страница выглядит скучно, вопрос лишь в том, зачем ты это читаешь.
        </p>

        <div class="mt-7 flex flex-wrap items-center justify-center gap-2">
          <span
              v-if="version"
              class="border border-line bg-ink-800 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-fg-muted"
          >
            v{{ version }}
          </span>
          <span class="border border-line bg-ink-800 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-fg-muted">
            Apache-2.0
          </span>
          <span
              v-if="tauriVersion"
              class="border border-line bg-ink-800 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-fg-muted"
          >
            Tauri {{ tauriVersion }}
          </span>
        </div>
      </header>

      <section class="animate-rise mt-14 w-full [animation-delay:80ms]">
        <SectionHeading index="01" title="Разработчик" meta="zaralX"/>

        <div class="mt-5 grid gap-3 sm:grid-cols-2">
          <button
              v-for="entry in contacts"
              :key="entry.label"
              type="button"
              class="group relative flex items-center gap-4 border border-line bg-ink-800 px-5 py-4 text-left transition-colors duration-500 hover:border-acid/50"
              @click="activate(entry)"
          >
            <UIcon
                :name="entry.icon"
                class="size-[18px] shrink-0 text-fg-faint transition-colors duration-300 group-hover:text-acid"
            />

            <span class="min-w-0 flex-1">
              <span class="block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
                {{ entry.label }}
              </span>
              <span class="mt-1.5 block truncate text-[13px] leading-none text-fg">{{ entry.value }}</span>
            </span>

            <UIcon
                :name="entry.url ? 'i-lucide-arrow-up-right' : 'i-lucide-copy'"
                class="size-3.5 shrink-0 text-fg-faint opacity-0 transition-all duration-300 ease-deck group-hover:opacity-100"
                :class="entry.url ? '-translate-x-1 group-hover:translate-x-0' : 'group-hover:text-fg-muted'"
            />
          </button>
        </div>
      </section>

      <section class="animate-rise mt-12 w-full [animation-delay:160ms]">
        <SectionHeading index="02" title="Проект" meta="Cast Launcher"/>

        <div class="mt-5 grid gap-3 sm:grid-cols-2">
          <button
              v-for="entry in project"
              :key="entry.label"
              type="button"
              class="group flex items-center gap-4 border border-line bg-ink-800 px-5 py-4 text-left transition-colors duration-500 hover:border-line-strong"
              @click="activate(entry)"
          >
            <UIcon
                :name="entry.icon"
                class="size-[18px] shrink-0 text-fg-faint transition-colors duration-300 group-hover:text-acid"
            />

            <span class="min-w-0 flex-1">
              <span class="block font-mono text-[10px] uppercase tracking-[0.24em] text-fg-faint">
                {{ entry.label }}
              </span>
              <span class="mt-1.5 block truncate text-[13px] leading-none text-fg">{{ entry.value }}</span>
            </span>

            <UIcon
                name="i-lucide-arrow-up-right"
                class="size-3.5 shrink-0 -translate-x-1 text-fg-faint opacity-0 transition-all duration-300 ease-deck group-hover:translate-x-0 group-hover:opacity-100"
            />
          </button>
        </div>

        <p class="mt-4 text-center text-[12px] leading-relaxed text-fg-muted">
          Пул реквесты приветствуются
        </p>
      </section>

      <section class="animate-rise mt-12 w-full [animation-delay:240ms]">
        <SectionHeading index="03" title="Стек" :meta="`${stack.length} шт.`"/>

        <div class="mt-5 flex flex-wrap justify-center gap-2">
          <span
              v-for="item in stack"
              :key="item"
              class="border border-line bg-ink-800 px-3 py-1.5 font-mono text-[10px] uppercase tracking-[0.18em] text-fg-muted transition-colors duration-300 hover:border-line-strong hover:text-fg"
          >
            {{ item }}
          </span>
        </div>
      </section>

      <section class="animate-rise mt-12 w-full [animation-delay:320ms]">
        <SectionHeading index="04" title="Спасибо"/>

        <ul class="mt-5 divide-y divide-line border border-line bg-ink-800">
          <li
              v-for="item in thanks"
              :key="item.name"
              class="flex flex-col gap-1 px-5 py-3.5 sm:flex-row sm:items-baseline sm:justify-between sm:gap-6"
          >
            <span class="text-[13px] leading-none text-fg">{{ item.name }}</span>
            <span class="text-[12px] leading-none text-fg-faint sm:text-right">{{ item.note }}</span>
          </li>
        </ul>
      </section>

      <footer class="animate-rise mt-14 flex flex-col items-center gap-3 text-center [animation-delay:400ms]">
        <span class="h-px w-24 bg-line"/>
        <p class="max-w-md font-mono text-[10px] uppercase leading-relaxed tracking-[0.18em] text-fg-faint">
          Не является официальным продуктом Minecraft. Не одобрено Mojang или Microsoft
          и никак с ними не связано.
        </p>
        <p class="font-mono text-[10px] uppercase tracking-[0.18em] text-fg-faint">
          Сделано с ♥ — zaralX
        </p>
      </footer>
    </div>
  </div>
</template>
