import type {InstanceType} from "~/types/instance";

export interface MyPacksConfig {
    packs: Record<string, MyPackObject>
}

export interface MyPackObject {
    id: string
    name: string
    description: string
    minecraftVersion: string
    type: InstanceType
    version: number
}