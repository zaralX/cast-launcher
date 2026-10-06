import type { ClassValue } from 'clsx'
import { clsx } from 'clsx'
import { extendTailwindMerge } from 'tailwind-merge'

// Without our theme keys tailwind-merge reads `text-label` as a color and drops it next to `text-fg`.
const twMerge = extendTailwindMerge({
  extend: {
    theme: {
      text: ['micro', 'label', 'caption', 'body', 'title', 'lead', 'heading'],
      tracking: ['caps', 'caps-wide', 'heading', 'display'],
      animate: ['rise', 'sweep', 'blink', 'breathe'],
      ease: ['deck'],
    },
  },
})

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
