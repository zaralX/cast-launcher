<script setup lang="ts">
import type { Instance, InstanceType } from '~/types/instance'
import { INSTANCE_TYPE_LABELS, INSTANCE_TYPE_MARKS, INSTANCE_TYPES } from '~/types/instance'

const props = defineProps<{ instance: Instance }>()

const instanceStore = useInstanceStore()
const toast = useAppToast()
const versions = useGameVersions()
const { loading, loadError } = versions

const { t } = useI18n()

const type = ref<InstanceType>(props.instance.type)
const minecraftVersion = ref(props.instance.minecraftVersion)
const loaderVersion = ref(props.instance.loaderVersion ?? '')

const applying = ref(false)
const confirmOpen = ref(false)

const running = computed(() => instanceStore.isRunning(props.instance.id))
const installing = computed(() => !!instanceStore.getInstall(props.instance.id))
const detaches = computed(() => !!props.instance.pack || !!props.instance.localPack)

const sameGame = computed(() =>
  type.value === props.instance.type && minecraftVersion.value === props.instance.minecraftVersion,
)

function isInstalledLoader(version: string) {
  const current = props.instance.loaderVersion
  return !!current && (version === current || version === `${minecraftVersion.value}-${current}`)
}

const minecraftItems = computed(() => {
  const list = versions.minecraft.value
  const current = props.instance.minecraftVersion
  return list.includes(current) ? list : [current, ...list]
})

const loaderItems = computed(() => {
  const list = versions.loaderVersions(type.value, minecraftVersion.value)
  const current = props.instance.loaderVersion

  if (!sameGame.value || !current || list.some(isInstalledLoader)) return list
  return [current, ...list]
})

watch(loaderItems, (list) => {
  if (list.includes(loaderVersion.value)) return
  loaderVersion.value = (sameGame.value ? list.find(isInstalledLoader) : undefined) ?? list[0] ?? ''
}, { immediate: true })

function reset() {
  type.value = props.instance.type
  minecraftVersion.value = props.instance.minecraftVersion
  loaderVersion.value = loaderItems.value.find(isInstalledLoader) ?? props.instance.loaderVersion ?? ''
}

watch(() => [props.instance.type, props.instance.minecraftVersion, props.instance.loaderVersion], reset)

const missingLoader = computed(() => type.value !== 'vanilla' && loaderItems.value.length === 0)

const changed = computed(() =>
  !sameGame.value || (type.value !== 'vanilla' && !isInstalledLoader(loaderVersion.value)),
)

const blocked = computed(() => {
  if (running.value) return t('instance.pack.blocked_running')
  if (installing.value) return t('instance.pack.blocked_installing')
  return null
})

const canApply = computed(() =>
  !loading.value && !loadError.value && !applying.value && !blocked.value && !missingLoader.value && changed.value,
)

function apply() {
  if (!canApply.value) return

  if (detaches.value) confirmOpen.value = true
  else run()
}

async function run() {
  applying.value = true

  const context = { instanceId: props.instance.id, action: t('instance.version.switch_action') }

  const switched = await attempt(
    () => call('set_instance_game_version', {
      instanceId: props.instance.id,
      change: {
        type: type.value,
        minecraftVersion: minecraftVersion.value,
        loaderVersion: type.value === 'vanilla' ? undefined : loaderVersion.value,
      },
    }),
    { context },
  )

  if (!switched.ok) {
    applying.value = false
    return
  }

  confirmOpen.value = false

  await safeRun(() => instanceStore.installInstance(props.instance.id), { code: 'NETWORK', context })

  applying.value = false

  toast.add({
    title: t('instance.version.switched', {
      loader: INSTANCE_TYPE_LABELS[switched.value.type],
      version: switched.value.minecraftVersion,
    }),
    color: 'success',
    icon: 'i-lucide-refresh-cw',
  })
}
</script>

<template>
  <div class="space-y-6">
    <KitPanel
      :title="$t('instance.version.title')"
      icon="i-lucide-git-branch"
    >
      <KitLoading
        v-if="loading"
        :label="$t('create.loading')"
        class="py-10"
      />

      <KitLoadError
        v-else-if="loadError"
        :error="loadError"
        @retry="versions.load"
      />

      <div
        v-else
        class="space-y-7"
      >
        <KitField :label="$t('create.loader')">
          <RadioGroup
            v-model="type"
            :aria-label="$t('create.loader')"
          >
            <RadioGroupItem
              v-for="item in INSTANCE_TYPES"
              :key="item"
              :value="item"
              class="flex-col gap-1.5 py-4"
            >
              <span class="font-mono text-caption tracking-[0.1em] transition-colors duration-300 group-data-[state=checked]:text-accent">
                {{ INSTANCE_TYPE_MARKS[item] }}
              </span>
              <span class="font-unbounded text-body tracking-[-0.02em]">{{ INSTANCE_TYPE_LABELS[item] }}</span>
            </RadioGroupItem>
          </RadioGroup>
        </KitField>

        <div
          class="grid gap-5"
          :class="type === 'vanilla' ? 'grid-cols-1' : 'grid-cols-2'"
        >
          <KitField label="Minecraft">
            <KitSelectMenu
              v-model="minecraftVersion"
              :items="minecraftItems"
              :search-placeholder="$t('instance.version.search')"
            />
          </KitField>

          <KitField
            v-if="type !== 'vanilla'"
            :label="INSTANCE_TYPE_LABELS[type]"
          >
            <KitSelectMenu
              v-model="loaderVersion"
              :items="loaderItems"
              :disabled="missingLoader"
              :placeholder="$t('create.no_builds')"
              :search-placeholder="$t('instance.version.search')"
            />
          </KitField>
        </div>

        <KitNote
          v-if="missingLoader"
          icon="i-lucide-triangle-alert"
        >
          {{ $t('instance.version.no_builds', { loader: INSTANCE_TYPE_LABELS[type], version: minecraftVersion }) }}
        </KitNote>

        <KitNote
          v-if="detaches"
          icon="i-lucide-unlink"
        >
          {{ $t('instance.version.detach_note') }}
        </KitNote>

        <KitRow :description="$t('instance.version.apply_hint')">
          <div class="flex gap-1.5">
            <Button
              v-if="changed && !applying"
              variant="ghost"
              @click="reset"
            >
              {{ $t('instance.version.reset') }}
            </Button>
            <Button
              icon="i-lucide-refresh-cw"
              :loading="applying"
              :disabled="!canApply"
              @click="apply"
            >
              {{ applying ? $t('instance.version.applying') : $t('instance.version.apply') }}
            </Button>
          </div>
        </KitRow>

        <KitStatus
          v-if="blocked"
          dot="static"
        >
          {{ blocked }}
        </KitStatus>
      </div>
    </KitPanel>

    <KitConfirmDialog
      v-model:open="confirmOpen"
      tone="default"
      confirm-icon="i-lucide-unlink"
      :title="$t('instance.version.detach_title')"
      :description="$t('instance.version.detach_text')"
      :confirm-label="$t('instance.version.detach_confirm')"
      :loading="applying"
      @confirm="run"
    />
  </div>
</template>
