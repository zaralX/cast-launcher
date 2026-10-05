export interface IconFile {
  name: string
  size: number
  modified: number
}

export interface ItemCatalog {
  categories: Record<string, string[]>
  names: Record<string, string>
}
