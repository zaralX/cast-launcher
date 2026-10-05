function copyWithSelection(text: string): boolean {
  const area = document.createElement('textarea')

  area.value = text
  area.setAttribute('readonly', '')
  area.style.position = 'fixed'
  area.style.top = '0'
  area.style.left = '0'
  area.style.opacity = '0'
  area.style.pointerEvents = 'none'

  document.body.appendChild(area)

  try {
    area.focus()
    area.select()
    area.setSelectionRange(0, text.length)

    return document.execCommand('copy')
  }
  catch {
    return false
  }
  finally {
    area.remove()
  }
}

export async function copyToClipboard(text: string): Promise<boolean> {
  if (copyWithSelection(text)) return true

  try {
    await navigator.clipboard.writeText(text)
    return true
  }
  catch {
    return false
  }
}
