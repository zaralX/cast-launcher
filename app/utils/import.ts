import type { ImportOptions } from '~/types/import'

export function defaultImportOptions(): ImportOptions {
  return {
    assets: true,
    libraries: true,
    java: true,
    icons: true,
    linkPacks: true,
  }
}
