export interface Account {
    type: 'offline' | 'microsoft'
    name: string
    uuid?: string
    accessToken?: string // Minecraft Access Token from @MinecraftAccount
    expiresAt?: number // accessToken expiresAt , in seconds , expiresIn обычно 86400 (24h)
    refreshToken?: string // Microsoft Refresh Token from @MicrosoftTokens
    xblHash?: string // XboxLive account hash
    skins?: MinecraftSkinData[]
    capes?: MinecraftCapeData[]
}

export interface AccountConfig {
    accounts: Account[]
    selected?: number
}

export interface MicrosoftTokens {
    access_token: string
    expires_in: number
    refresh_token: string
    scope: string
    token_type: string
    user_id: string
    error?: string
}

export interface XboxLiveResponse {
    Token: string
    DisplayClaims: {
        xui: { uhs: string }[]
    }
}

export interface MinecraftAccount {
    access_token: string
    expires_in: number
    metadata: {}
    roles: []
    token_type: string
    username: string // uuid
}

export interface MinecraftProfile {
    name: string
    id: string // uuid
    profileActions: {}
    skins: MinecraftSkinData[]
    capes: MinecraftCapeData[]
}

export interface MinecraftSkinData {
    id: string // uuid
    state: "ACTIVE" | "INACTIVE"
    textureKey: string
    url: string
    variant: "SLIM" | "CLASSIC"
}

export interface MinecraftCapeData {
    alias: string // name
    id: string // uuid
    state: "ACTIVE" | "INACTIVE"
    url: string
}