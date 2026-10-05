import type { AccountLook, Look } from '~/types/skin'

export function sameLook(a: Look, b: Look) {
  return a.skinId === b.skinId && a.capeId === b.capeId && a.variant === b.variant
}

export function lookOf(look: AccountLook): Look {
  return {
    skinId: look.skinId ?? null,
    capeId: look.capeId ?? null,
    variant: look.variant,
  }
}
