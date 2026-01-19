export interface Account {
    type: 'offline'
    name: string
    skin?: AccountSkin
}

export interface AccountSkin {
    data: string
    id: string
    url: string
    variant: "SLIM" | "CLASSIC"
}

export interface AccountConfig {
    accounts: Account[]
    selected?: number
}