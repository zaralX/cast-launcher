import { v4 } from 'uuid'
import type { ErrorEntry, ReportOptions } from '~/types/error'

type ErrorSink = (entry: ErrorEntry) => void

const sinks = new Set<ErrorSink>()

export function registerErrorSink(sink: ErrorSink) {
  sinks.add(sink)
  return () => sinks.delete(sink)
}

const MAX_ENTRIES = 50
const DEDUPE_WINDOW_MS = 5000

export const useErrorStore = defineStore('error', {
  state: () => ({
    entries: [] as ErrorEntry[],
    seenAt: 0,
  }),
  getters: {
    unseenCount: state => state.entries.filter(e => e.at > state.seenAt).length,
    lastEntry: state => state.entries[0],
  },
  actions: {
    report(raw: unknown, options: ReportOptions = {}): LauncherError {
      const error = toLauncherError(raw, options.code ?? 'UNKNOWN', options.context ?? {})

      console.error(`[${error.code}]`, error.message, error.context, error.cause ?? error)

      const previous = this.entries[0]
      if (
        previous
        && previous.code === error.code
        && previous.message === error.message
        && Date.now() - previous.at < DEDUPE_WINDOW_MS
      ) {
        previous.count += 1
        previous.at = Date.now()
        return error
      }

      trackEvent('ui_error', {
        code: error.code,
        severity: error.severity,
        command: String(error.context.command ?? ''),
        stage: String(error.context.stage ?? ''),
      })

      const entry: ErrorEntry = {
        id: v4(),
        at: Date.now(),
        code: error.code,
        severity: error.severity,
        icon: error.icon,
        title: error.title,
        message: error.message,
        hint: error.hint,
        details: error.details,
        context: error.context,
        report: error.toReport(),
        count: 1,
      }

      this.entries.unshift(entry)
      if (this.entries.length > MAX_ENTRIES) {
        this.entries.length = MAX_ENTRIES
      }

      if (options.toast !== false) {
        for (const sink of sinks) sink(entry)
      }

      return error
    },

    markAllSeen() {
      this.seenAt = Date.now()
    },

    dismiss(id: string) {
      this.entries = this.entries.filter(e => e.id !== id)
    },

    clear() {
      this.entries = []
      this.seenAt = Date.now()
    },
  },
})
