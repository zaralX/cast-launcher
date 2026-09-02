import {defineStore} from 'pinia'
import type {ImportProgress, ImportReport} from "~/types/import"

export const useImportStore = defineStore('import', {
    state: () => ({
        progress: null as null | ImportProgress,
        report: null as null | ImportReport,
        starting: false
    }),
    getters: {
        running: (state) => state.starting || state.progress !== null
    },
    actions: {
        applyBootstrap(progress: ImportProgress | null | undefined) {
            this.progress = progress ?? null
        },

        applyProgress(progress: ImportProgress) {
            this.starting = false
            this.progress = progress.stage === "done" ? null : progress
        },

        applyReport(report: ImportReport) {
            this.starting = false
            this.progress = null
            this.report = report
        },

        begin() {
            this.starting = true
            this.report = null
        },

        settle() {
            this.starting = false
        }
    }
})
