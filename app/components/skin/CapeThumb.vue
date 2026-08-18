<script setup lang="ts">
import {CAPE_BOX, CAPE_TEXTURE_SIZE, boxFaces} from "~/utils/skinGeometry"

// Наружная сторона плаща 10x16, без 3D. 
const props = withDefaults(defineProps<{
  cape: string
  scale?: number
}>(), {scale: 4})

const style = computed(() => {
  const rect = boxFaces(CAPE_BOX).front

  return {
    width: `${rect.w * props.scale}px`,
    height: `${rect.h * props.scale}px`,
    backgroundImage: `url("${props.cape}")`,
    backgroundSize: `${CAPE_TEXTURE_SIZE.w * props.scale}px ${CAPE_TEXTURE_SIZE.h * props.scale}px`,
    backgroundPosition: `${-rect.x * props.scale}px ${-rect.y * props.scale}px`
  }
})
</script>

<template>
  <div class="bg-no-repeat [image-rendering:pixelated]" :style="style"/>
</template>
