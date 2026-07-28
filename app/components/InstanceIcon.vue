<script setup lang="ts">
import type {InstanceType} from "~/types/instance";

const props = withDefaults(defineProps<{
  icon?: string
  type?: InstanceType
  size?: "sm" | "md" | "lg"
  bordered?: boolean
}>(), {icon: "", size: "sm", bordered: true})

const iconStore = useIconStore()

const TYPE_MARK: Record<InstanceType, string> = {
  vanilla: "VA",
  fabric: "FA",
  forge: "FO"
}

const SIZES = {
  sm: "size-8 text-[10px]",
  md: "size-12 text-[12px]",
  lg: "size-20 text-[16px]"
}

const mark = computed(() => (props.type && TYPE_MARK[props.type]) || "??")
const url = computed(() => iconStore.urlOf(props.icon))

/** Иконки предметов Minecraft — пиксель-арт, сглаживать их нельзя. */
const pixelated = computed(() => props.icon.startsWith("mc-"))

watch(() => props.icon, (name) => {
  if (name) iconStore.ensureUrl(name)
}, {immediate: true})
</script>

<template>
  <span
      class="grid shrink-0 place-items-center overflow-hidden"
      :class="[SIZES[size], bordered ? 'border border-line' : '']"
  >
    <img
        v-if="url"
        :src="url"
        alt=""
        class="size-full object-contain p-[3px]"
        :class="pixelated ? '[image-rendering:pixelated]' : ''"
    />
    <span v-else class="font-mono tracking-[0.08em]">{{ mark }}</span>
  </span>
</template>
