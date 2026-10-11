// Looks shared by several components in ui/ and kit/: a field, a floating panel, a menu row, a status tone.

import type { StatusTone } from '~/types/ui'

export const FOCUS_RING = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent'

export const FIELD = 'rounded-sm bg-control text-body text-fg outline-none transition-[box-shadow,background-color] duration-150 '
  + 'placeholder:text-fg-faint focus-visible:shadow-focus aria-invalid:shadow-invalid '
  + 'disabled:cursor-not-allowed disabled:opacity-45'

export const FLOATING = 'z-50 rounded-md bg-raise text-fg shadow-float ring-1 ring-line-strong outline-none duration-100 '
  + 'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 '
  + 'data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95'

export const MENU_ITEM = 'relative flex h-8 w-full cursor-pointer items-center gap-2.5 rounded-sm px-2.5 text-body text-fg outline-none select-none '
  + 'data-[highlighted]:bg-hover data-[state=open]:bg-hover data-[disabled]:cursor-not-allowed data-[disabled]:opacity-45 '
  + 'data-[variant=danger]:text-danger'

export const MENU_ICON = 'size-4 shrink-0 text-fg-faint in-data-[variant=danger]:text-danger'

// Leaves room on the right for the check of a selected item.
export const MENU_INDICATOR = 'pointer-events-none absolute right-2 flex items-center text-accent'

export const MENU_SEPARATOR = 'mx-1.5 my-1 bg-line-strong data-[orientation=horizontal]:w-auto'

export const MENU_LABEL = 'px-2.5 pt-2 pb-1 text-micro font-semibold text-fg-faint '
  + 'classic:font-mono classic:uppercase classic:tracking-caps'

export const STATUS_TONE: Record<StatusTone, string> = {
  muted: 'text-fg-faint',
  accent: 'text-accent',
  success: 'text-success',
  warning: 'text-warning',
  danger: 'text-danger',
}
