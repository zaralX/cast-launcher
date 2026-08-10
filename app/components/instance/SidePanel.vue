<script setup lang="ts">
import type {Instance} from "~/types/instance";
import {formatLastPlayed, formatPlaytime, INSTANCE_TYPE_LABELS} from "~/types/instance";
import type {InstanceDir} from "~/types/backend";

const props = defineProps<{
  instance: Instance | null
}>()

const emit = defineEmits<{
  remove: [id: string]
}>()

const actions = useInstanceActions()

const state = computed(() => props.instance ? actions.stateOf(props.instance) : null)
const install = computed(() => props.instance ? actions.installOf(props.instance.id) : undefined)

const {total} = usePlaytime(() => props.instance)

const playtime = computed(() => formatPlaytime(total.value) || "не запускалась")
const lastPlayed = computed(() => formatLastPlayed(props.instance?.playtime?.lastPlayedAt ?? 0))

const DIRS: InstanceDir[] = ["root", "minecraft", "logs"]

const DIR_ICONS: Record<InstanceDir, string> = {
  root: "i-lucide-folder-open",
  minecraft: "i-lucide-box",
  logs: "i-lucide-scroll-text"
}
</script>

<template>
  <aside class="flex flex-col border-l border-line bg-ink-800">
    <div v-if="instance" :key="instance.id" class="flex min-h-0 flex-1 flex-col">
      <div class="min-h-0 flex-1 overflow-y-auto">
        <header class="flex flex-col items-center gap-3 border-b border-line px-5 py-6 text-center">
          <InstanceIcon :icon="instance.icon" :type="instance.type" size="lg" class="text-fg-faint"/>

          <div class="min-w-0 w-full">
            <h3
                class="break-words font-unbounded text-[14px] font-semibold leading-tight tracking-[-0.045em] text-fg"
                :title="instance.name"
            >
              {{ instance.name }}
            </h3>
            <p class="mt-2 font-mono text-[9px] uppercase tracking-[0.18em] text-fg-faint">
              {{ INSTANCE_TYPE_LABELS[instance.type] ?? instance.type }} · {{ instance.minecraftVersion }}
            </p>
          </div>
        </header>

        <div class="space-y-3 px-5 py-5">
          <div v-if="state === 'installing'" class="flex h-9 flex-col justify-center gap-1.5">
            <span class="flex items-baseline justify-between gap-3">
              <span class="min-w-0 truncate font-mono text-[9px] uppercase tracking-[0.18em] text-fg-muted">
                {{ install?.phase || 'Установка' }}
              </span>
              <span v-if="install?.progress != null" class="shrink-0 font-mono text-[9px] tabular-nums text-acid">
                {{ Math.round(install.progress * 100) }}%
              </span>
            </span>

            <span class="relative block h-px w-full overflow-hidden bg-line">
              <span v-if="install?.progress == null" class="absolute inset-y-0 left-0 w-1/4 bg-acid animate-sweep"/>
              <span
                  v-else
                  class="absolute inset-y-0 left-0 bg-acid transition-[width] duration-500 ease-deck"
                  :style="{ width: `${Math.round(install.progress * 100)}%` }"
              />
            </span>
          </div>

          <AppButton
              v-else-if="state != 'running'"
              block
              class="group/act text-[10px] tracking-[0.18em]"
              @click="actions.primary(instance)"
          >
            <template #leading>
              <UIcon
                  :name="state === 'ready' ? 'i-lucide-play' : 'i-lucide-arrow-down-to-line'"
                  class="size-3 transition-transform duration-500 ease-deck group-hover/act:translate-x-0.5"
              />
            </template>
            {{ state === 'ready' ? 'Играть' : 'Загрузить' }}
          </AppButton>

          <AppButton
              v-if="state === 'running'"
              block
              class="h-8 text-[10px] tracking-[0.18em] border-acid/40 text-acid"
              icon="i-lucide-square"
              @click="actions.stop(instance.id)"
          >
            Остановить
          </AppButton>

          <AppButton
              v-if="state === 'installing'"
              block
              class="h-8 text-[10px] tracking-[0.18em]"
              icon="i-lucide-x"
              :disabled="install?.aborting"
              @click="actions.cancelInstall(instance.id)"
          >
            {{ install?.aborting ? 'Отмена…' : 'Отменить' }}
          </AppButton>

          <NuxtLink :to="`/instance/${instance.id}`" class="block">
            <AppButton block class="h-8 text-[10px] tracking-[0.18em]" icon="i-lucide-settings">
              Настройки
            </AppButton>
          </NuxtLink>

          <div class="grid grid-cols-3 gap-2">
            <button
                v-for="target in DIRS"
                :key="target"
                type="button"
                :title="INSTANCE_DIR_LABELS[target]"
                :aria-label="INSTANCE_DIR_LABELS[target]"
                class="grid h-8 cursor-pointer place-items-center border border-line text-fg-faint transition-colors duration-300 hover:border-acid hover:text-acid"
                @click="actions.openDir(instance.id, target)"
            >
              <UIcon :name="DIR_ICONS[target]" class="size-3.5"/>
            </button>
          </div>
        </div>

        <dl class="space-y-2 border-t border-line px-5 py-4 font-mono text-[10px] uppercase tracking-[0.14em]">
          <div>
            <dt class="text-fg-faint">Наиграно</dt>
            <dd class="min-w-0 truncate text-fg-muted">{{ playtime }}</dd>
          </div>
          <div v-if="lastPlayed">
            <dt class="text-fg-faint">Последний раз</dt>
            <dd class="min-w-0 truncate text-fg-muted">{{ lastPlayed }}</dd>
          </div>
          <div v-if="instance.castpack" class="flex items-baseline justify-between gap-3">
            <dt class="text-fg-faint">CastPack</dt>
            <dd class="min-w-0 truncate text-fg-muted">{{ instance.castpack.version || '—' }}</dd>
          </div>
        </dl>
      </div>

      <div class="border-t border-line px-5 py-4">
        <AppButton
            tone="quiet"
            class="text-[10px] tracking-[0.18em] hover:text-red-400"
            icon="i-lucide-trash-2"
            @click="emit('remove', instance.id)"
        >
          Удалить сборку
        </AppButton>
      </div>
    </div>

    <div v-else class="flex flex-1 flex-col items-center justify-center gap-3 px-5 py-14 text-center">
      <UIcon name="i-lucide-mouse-pointer-click" class="size-5 text-fg-faint"/>
      <p class="font-mono text-[10px] uppercase leading-relaxed tracking-[0.18em] text-fg-faint">
        Выберите сборку<br>левой кнопкой
      </p>
    </div>
  </aside>
</template>
