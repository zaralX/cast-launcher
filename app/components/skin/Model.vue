<script setup lang="ts">
import type {SkinPose, SkinVariant} from "~/types/skin"
import {
  CAPE_BOX,
  CAPE_TEXTURE_SIZE,
  MODEL_HEIGHT,
  SKIN_TEXTURE_SIZE,
  boxFaces,
  bodyParts,
  type FaceRect,
  type SkinBox
} from "~/utils/skinGeometry"

const props = withDefaults(defineProps<{
  skin: string
  cape?: string | null
  variant?: SkinVariant
  scale?: number
  pose?: SkinPose
  spinning?: boolean
  layers?: boolean
  interactive?: boolean
  angle?: number
  tilt?: number
}>(), {
  cape: null,
  variant: "CLASSIC",
  scale: 9,
  pose: "walk",
  spinning: false,
  layers: true,
  interactive: true,
  angle: 24,
  tilt: -6
})

const POSES: Record<SkinPose, { speed: number, swing: number, lean: number }> = {
  stand: {speed: 0, swing: 0, lean: 0},
  walk: {speed: 4.4, swing: 27, lean: 0},
  run: {speed: 7.6, swing: 44, lean: 12}
}

const MIN_ZOOM = 0.45
const MAX_ZOOM = 2.4

const yaw = ref(props.angle)
const pitch = ref(props.tilt)
const zoom = ref(1)
const phase = ref(0)

const motion = computed(() => POSES[props.pose])
const moving = computed(() => motion.value.speed > 0)

const unit = computed(() => props.scale * zoom.value)

const parts = computed(() => bodyParts(props.variant))

const playerStyle = computed(() => ({
  transform: `rotateX(${pitch.value + motion.value.lean}deg) rotateY(${yaw.value}deg)`
}))

const swingOf = (swing: number) => {
  if (!swing || !moving.value) return 0

  return Math.sin(phase.value * motion.value.speed) * motion.value.swing * swing
}

const headBob = computed(() =>
    moving.value ? Math.sin(phase.value * motion.value.speed * 2) * 2.2 : 0
)

const capeLean = computed(() =>
    8 + (moving.value ? Math.abs(Math.sin(phase.value * motion.value.speed)) * 16 : 0)
)

function jointStyle(jointX: number, jointY: number, rotate: number) {
  const u = unit.value

  return {
    transform: `translate3d(${jointX * u}px, ${(jointY - MODEL_HEIGHT / 2) * u}px, 0) rotateX(${rotate}deg)`
  }
}

function boxStyle(box: SkinBox, dir: 1 | -1, inflate = 0) {
  const u = unit.value

  return {
    width: `${box.w * u}px`,
    height: `${box.h * u}px`,
    transform: `translate(-50%, -50%) translateY(${dir * box.h / 2 * u}px)` +
        (inflate ? ` scale3d(${inflate}, ${inflate}, ${inflate})` : "")
  }
}

interface Face {
  key: string
  style: Record<string, string>
}

function faceList(box: SkinBox, url: string, texture: { w: number, h: number }): Face[] {
  const u = unit.value
  const {w, h, d} = box
  const rects = boxFaces(box)

  const skin = (rect: FaceRect, transform: string) => ({
    width: `${rect.w * u}px`,
    height: `${rect.h * u}px`,
    backgroundImage: `url("${url}")`,
    backgroundSize: `${texture.w * u}px ${texture.h * u}px`,
    backgroundPosition: `${-rect.x * u}px ${-rect.y * u}px`,
    transform: `translate(-50%, -50%) ${transform}`
  })

  return [
    {key: "front", style: skin(rects.front, `translateZ(${d / 2 * u}px)`)},
    {key: "back", style: skin(rects.back, `rotateY(180deg) translateZ(${d / 2 * u}px)`)},
    {key: "right", style: skin(rects.right, `rotateY(-90deg) translateZ(${w / 2 * u}px)`)},
    {key: "left", style: skin(rects.left, `rotateY(90deg) translateZ(${w / 2 * u}px)`)},
    {key: "top", style: skin(rects.top, `rotateX(90deg) translateZ(${h / 2 * u}px)`)},
    {key: "bottom", style: skin(rects.bottom, `rotateX(-90deg) translateZ(${h / 2 * u}px)`)}
  ]
}

const capeFaces = computed(() => props.cape ? faceList(CAPE_BOX, props.cape, CAPE_TEXTURE_SIZE) : [])

const capeJointStyle = computed(() => {
  const u = unit.value

  return {
    transform: `translate3d(0px, ${(8 - MODEL_HEIGHT / 2) * u}px, ${-2 * u}px) ` +
        `rotateX(${-capeLean.value}deg) rotateY(180deg)`
  }
})

const capeBoxStyle = computed(() => boxStyle(CAPE_BOX, 1))

// анимация

let frame = 0
let start = 0

const animating = computed(() => moving.value || props.spinning)

function loop(now: number) {
  if (!start) start = now

  const seconds = (now - start) / 1000

  if (moving.value) phase.value = seconds
  if (props.spinning) yaw.value = (yaw.value + 0.45) % 360

  frame = requestAnimationFrame(loop)
}

watch(animating, on => {
  cancelAnimationFrame(frame)
  frame = 0
  start = 0

  if (!on) {
    phase.value = 0
    return
  }

  frame = requestAnimationFrame(loop)
}, {immediate: true})

onBeforeUnmount(() => cancelAnimationFrame(frame))

// вращение мышкой

let dragging = false
let lastX = 0
let lastY = 0

function onPointerDown(event: PointerEvent) {
  if (!props.interactive) return

  dragging = true
  lastX = event.clientX
  lastY = event.clientY

  ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
}

function onPointerMove(event: PointerEvent) {
  if (!dragging) return

  yaw.value += (event.clientX - lastX) * 0.55
  pitch.value = Math.min(32, Math.max(-32, pitch.value - (event.clientY - lastY) * 0.35))

  lastX = event.clientX
  lastY = event.clientY
}

function onPointerUp(event: PointerEvent) {
  if (!dragging) return

  dragging = false
  ;(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId)
}

function onWheel(event: WheelEvent) {
  if (!props.interactive) return

  event.preventDefault()

  const next = zoom.value * (event.deltaY < 0 ? 1.12 : 1 / 1.12)
  zoom.value = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next))
}

function reset() {
  yaw.value = props.angle
  pitch.value = props.tilt
  zoom.value = 1
}

defineExpose({reset})
</script>

<template>
  <div
      class="relative grid h-full w-full place-items-center overflow-hidden select-none"
      :class="interactive ? 'cursor-grab active:cursor-grabbing' : ''"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
      @wheel="onWheel"
  >
    <div class="skin-scene relative" :style="{perspective: `${unit * 90}px`}">
      <div class="skin-3d absolute left-0 top-0" :style="playerStyle">
        <template v-for="part in parts" :key="part.key">
          <div
              class="skin-3d absolute left-0 top-0"
              :style="jointStyle(part.jointX, part.jointY, part.key === 'head' ? headBob : swingOf(part.swing))"
          >
            <div class="skin-3d absolute left-0 top-0" :style="boxStyle(part.box, part.dir)">
              <div
                  v-for="face in faceList(part.box, skin, SKIN_TEXTURE_SIZE)"
                  :key="face.key"
                  class="skin-face"
                  :style="face.style"
              />
            </div>

            <div
                v-if="layers && part.overlay"
                class="skin-3d absolute left-0 top-0"
                :style="boxStyle(part.overlay, part.dir, part.key === 'head' ? 1.11 : 1.06)"
            >
              <div
                  v-for="face in faceList(part.overlay, skin, SKIN_TEXTURE_SIZE)"
                  :key="face.key"
                  class="skin-face"
                  :style="face.style"
              />
            </div>
          </div>
        </template>

        <div v-if="cape" class="skin-3d absolute left-0 top-0" :style="capeJointStyle">
          <div class="skin-3d absolute left-0 top-0" :style="capeBoxStyle">
            <div
                v-for="face in capeFaces"
                :key="face.key"
                class="skin-face"
                :style="face.style"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.skin-scene {
  width: 0;
  height: 0;
}

.skin-3d {
  transform-style: preserve-3d;
  width: 0;
  height: 0;
}

.skin-face {
  position: absolute;
  left: 50%;
  top: 50%;
  image-rendering: pixelated;
  backface-visibility: hidden;
  background-repeat: no-repeat;
}
</style>
