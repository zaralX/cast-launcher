const DEFAULT_FACE = "/default_skin_face.png"

export function fallbackFace(event: Event) {
    const img = event.target as HTMLImageElement | null
    if (!img || img.dataset.fallback) return

    img.dataset.fallback = "1"
    img.src = DEFAULT_FACE
}
