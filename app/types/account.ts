export type AccountType = "offline" | "microsoft"

export interface Account {
    type: AccountType
    name: string
    uuid?: string
    accessToken?: string
    expiresAt?: number
    refreshToken?: string
    xblHash?: string
    skins: MinecraftSkinData[]
    capes: MinecraftCapeData[]
}

export interface AccountConfig {
    accounts: Account[]
    selected?: number
}

export interface MinecraftSkinData {
    id: string
    state: "ACTIVE" | "INACTIVE"
    textureKey: string
    url: string
    variant: "SLIM" | "CLASSIC"
}

export interface MinecraftCapeData {
    alias: string
    id: string
    state: "ACTIVE" | "INACTIVE"
    url: string
}
