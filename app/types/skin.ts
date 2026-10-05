export type SkinVariant = 'CLASSIC' | 'SLIM'

export type SkinSource = 'profile' | 'local' | 'player'

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

export type SkinPose = 'stand' | 'walk' | 'run'

export interface Look {
  skinId: string | null
  capeId: string | null
  variant: SkinVariant
}

export const VARIANT_LABELS: Record<SkinVariant, string> = {
  CLASSIC: 'Classic',
  SLIM: 'Slim',
}

export const VARIANT_HINT_KEYS: Record<SkinVariant, string> = {
  CLASSIC: 'skins.variant.classic_hint',
  SLIM: 'skins.variant.slim_hint',
}

export const SOURCE_KEYS: Record<SkinSource, string> = {
  profile: 'skins.source.profile',
  local: 'skins.source.local',
  player: 'skins.source.player',
}

export interface SkinBox {
  u: number
  v: number
  w: number
  h: number
  d: number
}

export interface FaceRect {
  x: number
  y: number
  w: number
  h: number
}

export interface SkinPart {
  key: string
  box: SkinBox
  overlay?: SkinBox
  jointX: number
  jointY: number
  dir: 1 | -1
  swing: number
}

export interface FlatLayer {
  key: string
  rect: FaceRect
  x: number
  y: number
}
