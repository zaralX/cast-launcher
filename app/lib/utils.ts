import type { ClassValue } from 'clsx'
import { clsx } from 'clsx'
import { extendTailwindMerge } from 'tailwind-merge'

// Without our theme keys tailwind-merge reads `text-caption` as a color and drops it next to `text-fg`.
const twMerge = extendTailwindMerge({
  extend: {
    theme: {
      text: ['micro', 'caption', 'body', 'title', 'lead', 'heading', 'page', 'display'],
      tracking: ['caps', 'caps-wide', 'heading', 'display'],
      radius: ['pill'],
      shadow: ['float', 'dialog', 'sheet', 'focus', 'invalid'],
      animate: ['rise', 'sweep', 'breathe'],
      ease: ['deck'],
    },
  },
})

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
