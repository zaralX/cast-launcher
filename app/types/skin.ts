export type SkinVariant = "CLASSIC" | "SLIM"

export type SkinSource = "profile" | "local" | "player"

export interface SkinEntry {
    id: string
    texture: string
    name: string
    variant: SkinVariant
    capeId?: string
    source: SkinSource
    addedAt: number
}

export interface SkinLibrary {
    skins: SkinEntry[]
}

export interface CapeView {
    id: string
    alias: string
    active: boolean
    texture?: string
}

export interface AccountLook {
    uuid: string
    name: string
    skinId?: string
    variant: SkinVariant
    capeId?: string
    capes: CapeView[]
    library: SkinLibrary
    stale: boolean
}

export type SkinPose = "stand" | "walk" | "run"

export interface Look {
    skinId: string | null
    capeId: string | null
    variant: SkinVariant
}

export const VARIANT_LABELS: Record<SkinVariant, string> = {
    CLASSIC: "Classic",
    SLIM: "Slim"
}

export const VARIANT_HINT_KEYS: Record<SkinVariant, string> = {
    CLASSIC: "skins.variant.classic_hint",
    SLIM: "skins.variant.slim_hint"
}

export const SOURCE_KEYS: Record<SkinSource, string> = {
    profile: "skins.source.profile",
    local: "skins.source.local",
    player: "skins.source.player"
}

export function sameLook(a: Look, b: Look) {
    return a.skinId === b.skinId && a.capeId === b.capeId && a.variant === b.variant
}

export function lookOf(look: AccountLook): Look {
    return {
        skinId: look.skinId ?? null,
        capeId: look.capeId ?? null,
        variant: look.variant
    }
}
