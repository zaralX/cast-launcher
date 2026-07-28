export interface IconFile {
    name: string
    size: number
    modified: number
}

export interface ItemCatalog {
    categories: Record<string, string[]>
    names: Record<string, string>
}

export const ITEM_CATEGORY_LABELS: Record<string, string> = {
    building_blocks: "Строительные блоки",
    colored_blocks: "Цветные блоки",
    natural_blocks: "Природные блоки",
    functional_blocks: "Функциональные блоки",
    redstone_blocks: "Редстоун",
    tools_and_utilities: "Инструменты",
    combat: "Оружие и броня",
    food_and_drinks: "Еда",
    ingredients: "Ресурсы",
    spawn_eggs: "Яйца призыва",
    op_blocks: "Операторские блоки"
}

export function itemCategoryLabel(key: string): string {
    return ITEM_CATEGORY_LABELS[key] ?? key.replace(/_/g, " ")
}

export function itemFallbackName(item: string): string {
    const words = item.replace(/_/g, " ")
    return words.charAt(0).toUpperCase() + words.slice(1)
}
