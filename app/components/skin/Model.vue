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
  scale: 1,
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

const BOUND_WIDTH = 19
const BOUND_HEIGHT = 37

const BAKE_DELAY = 180

const yaw = ref(props.angle)
const pitch = ref(props.tilt)
const zoom = ref(1)
const phase = ref(0)

const motion = computed(() => POSES[props.pose])
const moving = computed(() => motion.value.speed > 0)

const root = useTemplateRef<HTMLElement>("root")
const box = ref({width: 0, height: 0})

let resize: ResizeObserver | null = null

function measure(element: HTMLElement) {
  box.value = {width: element.clientWidth, height: element.clientHeight}
}

onMounted(() => {
  const element = root.value
  if (!element) return

  measure(element)

  resize = new ResizeObserver(() => measure(element))
  resize.observe(element)
})

onBeforeUnmount(() => resize?.disconnect())

const fitted = computed(() => {
  const {width, height} = box.value
  if (!width || !height) return 0

  return Math.min(width / BOUND_WIDTH, height / BOUND_HEIGHT)
})

const target = computed(() => fitted.value * props.scale * zoom.value)

const rendered = ref(0)

const unit = computed(() => rendered.value)

const viewScale = computed(() => rendered.value ? target.value / rendered.value : 1)

let bake = 0

watch(target, value => {
  if (!value) return

  if (!rendered.value) {
    rendered.value = value
    return
  }

  clearTimeout(bake)
  bake = window.setTimeout(() => (rendered.value = target.value), BAKE_DELAY)
}, {immediate: true})

onBeforeUnmount(() => clearTimeout(bake))

const parts = computed(() => bodyParts(props.variant))

const viewStyle = computed(() => ({transform: `scale(${viewScale.value})`}))

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

function boxStyle(box: SkinBox, dir: 1 | -1, u: number, inflate = 0) {
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

function faceList(box: SkinBox, url: string, texture: { w: number, h: number }, u: number): Face[] {
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

const partViews = computed(() => {
  const u = unit.value

  return parts.value.map(part => ({
    key: part.key,
    jointX: part.jointX,
    jointY: part.jointY,
    swing: part.swing,
    box: boxStyle(part.box, part.dir, u),
    faces: faceList(part.box, props.skin, SKIN_TEXTURE_SIZE, u),
    overlay: props.layers && part.overlay
        ? {
          box: boxStyle(part.overlay, part.dir, u, part.key === "head" ? 1.11 : 1.06),
          faces: faceList(part.overlay, props.skin, SKIN_TEXTURE_SIZE, u)
        }
        : null
  }))
})

const capeFaces = computed(() =>
    props.cape ? faceList(CAPE_BOX, props.cape, CAPE_TEXTURE_SIZE, unit.value) : []
)

const capeJointStyle = computed(() => {
  const u = unit.value

  return {
    transform: `translate3d(0px, ${(8 - MODEL_HEIGHT / 2) * u}px, ${-2 * u}px) ` +
        `rotateX(${-capeLean.value}deg) rotateY(180deg)`
  }
})

const capeBoxStyle = computed(() => boxStyle(CAPE_BOX, 1, unit.value))

const perspective = computed(() => `${Math.max(1, unit.value * 90)}px`)

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

const WHEEL_STEP = 100

let wheelFrame = 0
let wheelDelta = 0

function applyWheel() {
  wheelFrame = 0

  const delta = Math.max(-3 * WHEEL_STEP, Math.min(3 * WHEEL_STEP, wheelDelta))
  wheelDelta = 0

  const next = zoom.value * Math.pow(1.12, -delta / WHEEL_STEP)
  zoom.value = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next))
}

function onWheel(event: WheelEvent) {
  if (!props.interactive) return

  event.preventDefault()

  const step = event.deltaMode === 1 ? WHEEL_STEP / 3 : event.deltaMode === 2 ? WHEEL_STEP : 1

  wheelDelta += event.deltaY * step

  if (!wheelFrame) wheelFrame = requestAnimationFrame(applyWheel)
}

onBeforeUnmount(() => cancelAnimationFrame(wheelFrame))

function reset() {
  yaw.value = props.angle
  pitch.value = props.tilt
  zoom.value = 1
}

defineExpose({reset})
</script>

<template>
  <div
      ref="root"
      class="relative grid h-full w-full place-items-center overflow-hidden select-none"
      :class="interactive ? 'cursor-grab active:cursor-grabbing' : ''"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
      @wheel="onWheel"
  >
    <div class="skin-view relative" :style="viewStyle">
      <div class="skin-scene relative" :style="{perspective}">
        <div class="skin-3d absolute left-0 top-0" :style="playerStyle">
          <template v-for="part in partViews" :key="part.key">
            <div
                class="skin-3d absolute left-0 top-0"
                :style="jointStyle(part.jointX, part.jointY, part.key === 'head' ? headBob : swingOf(part.swing))"
            >
              <div class="skin-3d absolute left-0 top-0" :style="part.box">
                <div
                    v-for="face in part.faces"
                    :key="face.key"
                    class="skin-face"
                    :style="face.style"
                />
              </div>

              <div
                  v-if="part.overlay"
                  class="skin-3d absolute left-0 top-0"
                  :style="part.overlay.box"
              >
                <div
                    v-for="face in part.overlay.faces"
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
  </div>
</template>

<style scoped>
.skin-view,
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
