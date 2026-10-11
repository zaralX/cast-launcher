const DEFAULT_FACE = '/default_skin_face.png'

export function fallbackFace(event: Event) {
  const img = event.target as HTMLImageElement | null
  if (!img || img.dataset.fallback) return

  img.dataset.fallback = '1'
  img.src = DEFAULT_FACE
}

export function faceUrl(name: string): string {
  return `https://assets.zaralx.ru/api/v2/minecraft/players/${encodeURIComponent(name)}/face`
}
