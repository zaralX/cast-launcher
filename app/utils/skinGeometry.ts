import type {SkinVariant} from "~/types/skin"

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

export const MODEL_HEIGHT = 32

export const MODEL_WIDTH = 16

export const SKIN_TEXTURE_SIZE = {w: 64, h: 64}

export const CAPE_TEXTURE_SIZE = {w: 64, h: 32}

export const CAPE_BOX: SkinBox = {u: 0, v: 0, w: 10, h: 16, d: 1}

export function boxFaces(box: SkinBox): Record<"top" | "bottom" | "right" | "front" | "left" | "back", FaceRect> {
    const {u, v, w, h, d} = box

    return {
        top: {x: u + d, y: v, w, h: d},
        bottom: {x: u + d + w, y: v, w, h: d},
        right: {x: u, y: v + d, w: d, h},
        front: {x: u + d, y: v + d, w, h},
        left: {x: u + d + w, y: v + d, w: d, h},
        back: {x: u + d + w + d, y: v + d, w, h}
    }
}

export const armWidth = (variant: SkinVariant) => variant === "SLIM" ? 3 : 4

export function bodyParts(variant: SkinVariant): SkinPart[] {
    const arm = armWidth(variant)
    const armX = 4 + arm / 2

    return [
        {
            key: "head",
            box: {u: 0, v: 0, w: 8, h: 8, d: 8},
            overlay: {u: 32, v: 0, w: 8, h: 8, d: 8},
            jointX: 0,
            jointY: 8,
            dir: -1,
            swing: 0
        },
        {
            key: "body",
            box: {u: 16, v: 16, w: 8, h: 12, d: 4},
            overlay: {u: 16, v: 32, w: 8, h: 12, d: 4},
            jointX: 0,
            jointY: 8,
            dir: 1,
            swing: 0
        },
        {
            key: "armRight",
            box: {u: 40, v: 16, w: arm, h: 12, d: 4},
            overlay: {u: 40, v: 32, w: arm, h: 12, d: 4},
            jointX: -armX,
            jointY: 8,
            dir: 1,
            swing: 1
        },
        {
            key: "armLeft",
            box: {u: 32, v: 48, w: arm, h: 12, d: 4},
            overlay: {u: 48, v: 48, w: arm, h: 12, d: 4},
            jointX: armX,
            jointY: 8,
            dir: 1,
            swing: -1
        },
        {
            key: "legRight",
            box: {u: 0, v: 16, w: 4, h: 12, d: 4},
            overlay: {u: 0, v: 32, w: 4, h: 12, d: 4},
            jointX: -2,
            jointY: 20,
            dir: 1,
            swing: -1
        },
        {
            key: "legLeft",
            box: {u: 16, v: 48, w: 4, h: 12, d: 4},
            overlay: {u: 0, v: 48, w: 4, h: 12, d: 4},
            jointX: 2,
            jointY: 20,
            dir: 1,
            swing: 1
        }
    ]
}

export interface FlatLayer {
    key: string
    rect: FaceRect
    x: number
    y: number
}

export function frontLayers(variant: SkinVariant): FlatLayer[] {
    const arm = armWidth(variant)

    const layers: FlatLayer[] = []

    const push = (key: string, box: SkinBox, x: number, y: number) => {
        layers.push({key, rect: boxFaces(box).front, x, y})
    }

    for (const part of bodyParts(variant)) {
        const x = MODEL_WIDTH / 2 + part.jointX - part.box.w / 2
        const y = part.dir === -1 ? part.jointY - part.box.h : part.jointY

        push(part.key, part.box, x, y)
        if (part.overlay) push(`${part.key}-overlay`, part.overlay, x, y)
    }

    return layers
}
