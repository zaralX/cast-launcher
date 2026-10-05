import type { ExportProgress } from '~/types/cast'

export const useCastStore = defineStore('cast', {
  state: () => ({
    // .cast files the system asked the launcher to open, shown one by one.
    opened: [] as string[],
    exporting: null as ExportProgress | null,
  }),
  getters: {
    current: state => state.opened[0] ?? null,
  },
  actions: {
    async takeOpened() {
      const files = await call('take_opened_files')

      for (const file of files) {
        if (!this.opened.includes(file)) this.opened.push(file)
      }
    },

    dismissOpened() {
      this.opened.shift()
    },

    applyExportProgress(progress: ExportProgress) {
      this.exporting = progress
    },

    finishExport() {
      this.exporting = null
    },
  },
})
