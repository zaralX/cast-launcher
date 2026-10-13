<script setup lang="ts">
import type { Instance } from '~/types/instance'
import type { TabItem } from '~/types/ui'
import { useIntersectionObserver, useScroll } from '@vueuse/core'
import { INSTANCE_TYPE_LABELS, PACK_PROVIDER_LABELS } from '~/types/instance'
import { PROVIDER_LOGOS } from '~/types/catalog'

const BAR_HEIGHT = 44

const props = defineProps<{
  instance: Instance
  icon: string
  sections: TabItem[]
  tools: TabItem[]
}>()

const emit = defineEmits<{ export: [] }>()

const iconStore = useIconStore()
const compact = useCompact()

const tint = useImageTint(() => iconStore.urlOf(props.icon))
const tintStyle = computed(() => (tint.value ? { '--tint': tint.value } : undefined))

const { total } = usePlaytime(() => props.instance)
const playtime = computed(() => formatPlaytime(total.value))
const lastPlayed = computed(() => formatLastPlayed(props.instance.playtime?.lastPlayedAt ?? 0))

const loader = computed(() => {
  const label = INSTANCE_TYPE_LABELS[props.instance.type] ?? props.instance.type
  return props.instance.loaderVersion ? `${label} ${props.instance.loaderVersion}` : label
})

const pack = computed(() => {
  const { castpack, pack, localPack } = props.instance
  if (castpack) {
    return {
      logo: '/logo.svg',
      label: `${castpack.origin === 'file' ? '.cast' : 'CastPack'} ${castpack.version}`.trim(),
    }
  }
  if (pack) {
    return {
      logo: PROVIDER_LOGOS[pack.provider],
      label: `${PACK_PROVIDER_LABELS[pack.provider]} ${pack.versionNumber}`.trim(),
    }
  }
  if (localPack) {
    return {
      logo: null,
      label: `${localPack.name} ${localPack.version}`.trim(),
    }
  }
  return null
})

const hero = ref<HTMLElement | null>(null)
const bar = ref<HTMLElement | null>(null)
const scroller = ref<HTMLElement | null>(null)
const stuck = ref(false)

onMounted(() => {
  scroller.value = bar.value?.closest<HTMLElement>('[data-main-scroll]') ?? null
})

useIntersectionObserver(hero, ([entry]) => {
  stuck.value = !!entry && !entry.isIntersecting
}, { root: scroller, rootMargin: `-${BAR_HEIGHT}px 0px 0px 0px` })

watch(compact, () => {
  stuck.value = false
})

const { y } = useScroll(scroller)

const condensed = computed(() => compact.value || stuck.value)
const lifted = computed(() => (compact.value ? y.value > 0 : stuck.value))
</script>

<template>
  <header
    v-if="!compact"
    class="bg-tint px-8 pt-3.5"
    :style="tintStyle"
  >
    <Button
      to="/main"
      variant="ghost"
      size="sm"
      icon="i-lucide-arrow-left"
      class="-ml-2.5"
    >
      {{ $t('instance.back') }}
    </Button>

    <div
      ref="hero"
      class="mt-2.5 flex items-end gap-5 pb-3"
    >
      <InstanceIcon
        :icon="icon"
        :type="instance.type"
        :version="instance.minecraftVersion"
        size="2xl"
        class="ring-1 ring-line"
      />

      <div class="min-w-0 flex-1 pb-0.5">
        <KitTitle
          size="lg"
          class="truncate"
          :title="instance.name"
        >
          {{ instance.name }}
        </KitTitle>

        <div class="mt-2.5 flex flex-wrap items-center gap-2 text-fg-muted">
          <Badge
            v-if="pack"
            size="md"
          >
            <img
              v-if="pack.logo"
              :src="pack.logo"
              alt=""
              class="size-3.25"
            >
            {{ pack.label }}
          </Badge>
          <span>{{ loader }}</span>
          <span class="text-fg-faint">·</span>
          <span>Minecraft {{ instance.minecraftVersion }}</span>
          <template v-if="playtime">
            <span class="text-fg-faint">·</span>
            <span
              class="flex items-center gap-1.5"
              :title="$t('instance.playtime.title', { playtime })"
            >
              <Icon
                name="i-lucide-timer"
                class="size-3.5"
              />
              {{ playtime }}
            </span>
          </template>
          <template v-if="lastPlayed">
            <span class="text-fg-faint">·</span>
            <span :title="$t('instance.panel.last_played')">{{ lastPlayed }}</span>
          </template>
        </div>
      </div>

      <div class="flex shrink-0 items-center gap-2 pb-1">
        <InstanceMoreMenu
          :instance="instance"
          size="lg"
          @export="emit('export')"
        />

        <InstancePlayButton
          :instance="instance"
          size="lg"
        />
      </div>
    </div>
  </header>

  <div
    ref="bar"
    class="sticky top-0 z-20 flex h-11 items-stretch gap-3 border-b border-line bg-tint px-8 transition-shadow duration-150"
    :class="lifted && 'shadow-float'"
    :style="tintStyle"
  >
    <Transition
      enter-active-class="transition duration-150"
      leave-active-class="transition duration-100"
      enter-from-class="-translate-x-1 opacity-0"
      leave-to-class="opacity-0"
    >
      <div
        v-if="condensed"
        class="-ml-2.5 flex min-w-0 shrink-0 items-center gap-2.5 pr-2"
      >
        <Button
          to="/main"
          variant="ghost"
          size="icon"
          icon="i-lucide-arrow-left"
          :aria-label="$t('instance.back')"
        />
        <InstanceIcon
          :icon="icon"
          :type="instance.type"
          size="sm"
        />
        <span class="max-w-56 truncate font-unbounded text-title font-semibold tracking-heading classic:uppercase">
          {{ instance.name }}
        </span>
      </div>
    </Transition>

    <TabsList class="min-w-0 flex-1">
      <TabsTrigger
        v-for="section in sections"
        :key="section.value"
        :value="section.value"
        :icon="section.icon"
        :meta="section.meta"
      >
        {{ section.label }}
      </TabsTrigger>
      <span
        class="flex-1"
        aria-hidden="true"
      />
      <TabsTrigger
        v-for="section in tools"
        :key="section.value"
        :value="section.value"
        :icon="section.icon"
        :meta="section.meta"
      >
        {{ section.label }}
      </TabsTrigger>
    </TabsList>

    <Transition
      enter-active-class="transition duration-150"
      leave-active-class="transition duration-100"
      enter-from-class="opacity-0"
      leave-to-class="opacity-0"
    >
      <div
        v-if="condensed"
        class="flex shrink-0 items-center gap-1.5"
      >
        <InstanceMoreMenu
          v-if="compact"
          :instance="instance"
          @export="emit('export')"
        />
        <InstancePlayButton :instance="instance" />
      </div>
    </Transition>
  </div>
</template>
