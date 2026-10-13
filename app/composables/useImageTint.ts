import type { MaybeRefOrGetter } from 'vue'

const SAMPLE = 24

async function sample(url: string): Promise<string | null> {
  const image = new Image()
  image.src = url

  try {
    await image.decode()
  }
  catch {
    return null
  }

  const canvas = document.createElement('canvas')
  canvas.width = SAMPLE
  canvas.height = SAMPLE

  const context = canvas.getContext('2d', { willReadFrequently: true })
  if (!context) return null

  context.drawImage(image, 0, 0, SAMPLE, SAMPLE)

  let data: Uint8ClampedArray
  try {
    data = context.getImageData(0, 0, SAMPLE, SAMPLE).data
  }
  catch {
    return null
  }

  let red = 0
  let green = 0
  let blue = 0
  let total = 0

  for (let i = 0; i < data.length; i += 4) {
    const r = data[i]!
    const g = data[i + 1]!
    const b = data[i + 2]!
    const max = Math.max(r, g, b)
    const saturation = max ? (max - Math.min(r, g, b)) / max : 0
    const weight = (data[i + 3]! / 255) * (0.05 + saturation)

    red += r * weight
    green += g * weight
    blue += b * weight
    total += weight
  }

  if (!total) return null

  return `rgb(${Math.round(red / total)} ${Math.round(green / total)} ${Math.round(blue / total)})`
}

export function useImageTint(source: MaybeRefOrGetter<string | null | undefined>) {
  const tint = ref<string | null>(null)
  let request = 0

  watch(() => toValue(source), async (url) => {
    const id = ++request
    const color = url ? await sample(url) : null
    if (id === request) tint.value = color
  }, { immediate: true })

  return tint
}
