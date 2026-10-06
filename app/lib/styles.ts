// Shared looks of the primitives in components/ui: a field, a floating panel, a menu row.

export const FIELD = 'rounded-none bg-ink-900 text-lead text-fg ring ring-inset ring-line outline-none transition-colors '
  + 'placeholder:text-fg-faint focus-visible:ring-2 focus-visible:ring-acid/60 '
  + 'disabled:cursor-not-allowed disabled:opacity-75 aria-invalid:ring-danger/60'

export const FLOATING = 'z-50 border border-line bg-ink-800 text-fg shadow-lg outline-none duration-100 '
  + 'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 '
  + 'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95'

export const MENU_ITEM = 'relative flex w-full cursor-pointer items-center gap-1.5 p-1.5 text-fg-muted outline-none select-none transition-colors '
  + 'data-[highlighted]:bg-ink-700 data-[highlighted]:text-fg '
  + 'data-[disabled]:cursor-not-allowed data-[disabled]:opacity-75'
