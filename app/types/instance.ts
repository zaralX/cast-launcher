export interface Instance {
    id: string
    name: string
    description: string
    minecraftVersion: string
    type: InstanceType
    installed: boolean
    version: number
}

export type InstanceType = "vanilla" | "fabric" | "forge"