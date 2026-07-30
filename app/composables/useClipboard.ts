export async function copyToClipboard(text: string): Promise<boolean> {
    if (import.meta.client && document.body) {
        const area = document.createElement("textarea")

        area.value = text
        area.setAttribute("readonly", "")
        area.style.position = "fixed"
        area.style.top = "0"
        area.style.left = "0"
        area.style.opacity = "0"
        area.style.pointerEvents = "none"

        document.body.appendChild(area)

        try {
            area.focus()
            area.select()
            area.setSelectionRange(0, text.length)

            if (document.execCommand("copy")) return true
        } catch {
            // Анлак. Пробуем через навигатор копировать
        } finally {
            area.remove()
        }
    }

    try {
        await navigator.clipboard.writeText(text)
        return true
    } catch {
        return false
    }
}
