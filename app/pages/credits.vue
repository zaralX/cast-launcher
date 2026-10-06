<script setup lang="ts">
import { getVersion, getTauriVersion } from '@tauri-apps/api/app'

definePageMeta({
  layout: 'main',
})

type Entry = {
  label: string
  value: string
  icon: string
  url?: string
  copy?: string
}

const REPO = 'https://github.com/zaralX/cast-launcher'

const { t } = useI18n()

const contacts = computed<Entry[]>(() => [
  { label: t('credits.contacts.site'), value: 'zaralx.ru', icon: 'i-lucide-globe', url: 'https://zaralx.ru' },
  { label: 'GitHub', value: '@zaralX', icon: 'i-simple-icons-github', url: 'https://github.com/zaralX' },
  { label: 'Telegram', value: '@zWork1', icon: 'i-simple-icons-telegram', url: 'https://t.me/zWork1' },
  { label: t('credits.contacts.mail'), value: 'admin@zaralx.ru', icon: 'i-lucide-mail', copy: 'admin@zaralx.ru' },
])

const project = computed<Entry[]>(() => [
  { label: t('credits.project.sources'), value: 'zaralX/cast-launcher', icon: 'i-lucide-code-xml', url: REPO },
  { label: t('credits.project.issues'), value: 'Issues', icon: 'i-lucide-bug', url: `${REPO}/issues` },
  { label: t('credits.project.releases'), value: 'Releases', icon: 'i-lucide-package', url: `${REPO}/releases` },
  { label: t('credits.project.license'), value: 'Apache-2.0', icon: 'i-lucide-scale', url: `${REPO}/blob/main/LICENSE` },
])

const stack = [
  'Rust', 'Tauri 2', 'Nuxt 4', 'Vue 3', 'TypeScript', 'Tailwind CSS', 'shadcn-vue', 'Reka UI', 'Pinia',
]

const thanks = computed(() => [
  { name: 'Mojang Studios', note: t('credits.thanks.mojang') },
  { name: 'Modrinth', note: t('credits.thanks.modrinth') },
  { name: 'PrismLauncher', note: t('credits.thanks.prism') },
  { name: t('credits.thanks.testers_name'), note: t('credits.thanks.testers') },
])

const toast = useAppToast()

const version = ref('')
const tauriVersion = ref('')

onMounted(async () => {
  try {
    version.value = await getVersion()
  }
  catch {
    version.value = ''
  }

  try {
    tauriVersion.value = await getTauriVersion()
  }
  catch {
    tauriVersion.value = ''
  }
})

async function activate(entry: Entry) {
  if (entry.url) {
    await safeRun(() => call('open_url', { url: entry.url! }), {
      context: { action: t('credits.open_action'), url: entry.url },
    })
    return
  }

  if (!entry.copy) return

  const copied = await copyToClipboard(entry.copy)

  toast.add({
    title: copied ? t('credits.copied') : t('common.copy_failed'),
    description: entry.copy,
    color: copied ? 'success' : 'error',
    icon: copied ? 'i-lucide-clipboard-check' : 'i-lucide-clipboard-x',
  })
}
</script>

<template>
  <div class="relative min-h-full w-full overflow-hidden px-6 pt-14 pb-16 xl:px-10">
    <div
      class="pointer-events-none absolute top-0 left-1/2 size-80 -translate-x-1/2 -translate-y-1/3 rounded-full bg-acid/[0.07] blur-[120px]"
      aria-hidden="true"
    />

    <div class="relative mx-auto flex w-full max-w-3xl flex-col items-center">
      <header class="animate-rise flex flex-col items-center text-center">
        <div class="cut-16 relative grid size-20 place-items-center border border-line bg-ink-800">
          <img
            src="/logo.svg"
            class="size-11"
            alt="Cast Launcher"
          >
          <span class="absolute bottom-0 left-0 h-px w-full bg-gradient-to-r from-transparent via-acid to-transparent" />
        </div>

        <p class="mt-7 font-mono text-label uppercase tracking-caps-wide text-fg-faint">
          {{ $t('credits.eyebrow') }}
        </p>

        <h1 class="mt-4 font-unbounded text-[clamp(30px,6vw,52px)] font-bold leading-[0.9] tracking-[-0.06em] text-fg">
          CAST<span class="text-acid">.</span>LAUNCHER
        </h1>

        <p class="mt-5 max-w-md text-body leading-relaxed text-fg-muted">
          {{ $t('credits.intro') }}
        </p>

        <div class="mt-7 flex flex-wrap items-center justify-center gap-2">
          <Badge
            v-if="version"
            variant="surface"
            size="md"
          >
            v{{ version }}
          </Badge>
          <Badge
            variant="surface"
            size="md"
          >
            Apache-2.0
          </Badge>
          <Badge
            v-if="tauriVersion"
            variant="surface"
            size="md"
          >
            Tauri {{ tauriVersion }}
          </Badge>
        </div>
      </header>

      <section class="animate-rise mt-14 w-full [animation-delay:80ms]">
        <KitSectionHeading
          index="01"
          :title="$t('credits.developer')"
          meta="zaralX"
        />

        <div class="mt-5 grid grid-cols-2 gap-3">
          <CreditsLink
            v-for="entry in contacts"
            :key="entry.label"
            :label="entry.label"
            :value="entry.value"
            :icon="entry.icon"
            :copy="!entry.url"
            @click="activate(entry)"
          />
        </div>
      </section>

      <section class="animate-rise mt-12 w-full [animation-delay:160ms]">
        <KitSectionHeading
          index="02"
          :title="$t('credits.project_title')"
          meta="Cast Launcher"
        />

        <div class="mt-5 grid grid-cols-2 gap-3">
          <CreditsLink
            v-for="entry in project"
            :key="entry.label"
            :label="entry.label"
            :value="entry.value"
            :icon="entry.icon"
            @click="activate(entry)"
          />
        </div>

        <p class="mt-4 text-center text-body leading-relaxed text-fg-muted">
          {{ $t('credits.pull_requests') }}
        </p>
      </section>

      <section class="animate-rise mt-12 w-full [animation-delay:240ms]">
        <KitSectionHeading
          index="03"
          :title="$t('credits.stack')"
          :meta="$t('credits.stack_meta', { count: stack.length })"
        />

        <div class="mt-5 flex flex-wrap justify-center gap-2">
          <Badge
            v-for="item in stack"
            :key="item"
            variant="surface"
            size="md"
            class="transition-colors duration-300 hover:border-line-strong hover:text-fg"
          >
            {{ item }}
          </Badge>
        </div>
      </section>

      <section class="animate-rise mt-12 w-full [animation-delay:320ms]">
        <KitSectionHeading
          index="04"
          :title="$t('credits.thanks_title')"
        />

        <ul class="mt-5 divide-y divide-line border border-line bg-ink-800">
          <li
            v-for="item in thanks"
            :key="item.name"
            class="flex items-baseline justify-between gap-6 px-5 py-3.5"
          >
            <span class="text-title leading-none text-fg">{{ item.name }}</span>
            <span class="text-right text-body leading-none text-fg-faint">{{ item.note }}</span>
          </li>
        </ul>
      </section>

      <footer class="animate-rise mt-14 flex flex-col items-center gap-3 text-center [animation-delay:400ms]">
        <Separator class="w-24" />
        <p class="max-w-md font-mono text-label uppercase leading-relaxed tracking-caps text-fg-faint">
          {{ $t('credits.disclaimer') }}
        </p>
        <p class="font-mono text-label uppercase tracking-caps text-fg-faint">
          {{ $t('credits.made_by') }}
        </p>
      </footer>
    </div>
  </div>
</template>
