import { check } from '@tauri-apps/plugin-updater'
import type { Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import type { AppConfig, AppUpdate, AppUpdateProgress, JavaRuntime, LauncherPaths } from '~/types/app'

const CHECK_TIMEOUT = 20_000

const DOWNLOAD_TIMEOUT = 600_000

let found: Update | null = null

export const useAppStore = defineStore('app', {
  state: () => ({
    config: null as null | AppConfig,
    paths: null as null | LauncherPaths,
    javaRuntimes: [] as JavaRuntime[],
    javaScanning: false,
    javaScanned: false,
    update: null as AppUpdate | null,
    updateProgress: null as AppUpdateProgress | null,
    updateBusy: null as 'checking' | 'downloading' | 'installing' | null,
  }),
  getters: {
    hasConfig: state => !!state.config,

    needsOnboarding: state => state.config?.launcher.onboarded === false,

    systemJavaRuntime: (state): JavaRuntime | null =>
      state.javaRuntimes.find(runtime => runtime.source === 'path' || runtime.source === 'java_home')
      ?? state.javaRuntimes[0]
      ?? null,
  },
  actions: {
    applyBootstrap(config: AppConfig, paths: LauncherPaths) {
      this.config = config
      this.paths = paths
    },

    async updateConfig(config: AppConfig) {
      const rootBefore = this.paths?.root

      this.config = await call('update_config', { config })
      this.paths = await call('get_paths')

      setTelemetryEnabled(this.config.launcher.telemetry)

      this.javaScanned = false

      if (rootBefore !== undefined && rootBefore !== this.paths.root) {
        useIconStore().forgetLibrary()
        await useAccountStore().reload()
      }
    },

    async finishOnboarding() {
      if (!this.config || this.config.launcher.onboarded) return

      await this.updateConfig({
        ...this.config,
        launcher: { ...this.config.launcher, onboarded: true },
      })
    },

    async scanJava(force = false): Promise<JavaRuntime[]> {
      if (this.javaScanning) return this.javaRuntimes
      if (this.javaScanned && !force) return this.javaRuntimes

      this.javaScanning = true

      try {
        this.javaRuntimes = await call('list_java', { force })
        this.javaScanned = true
      }
      finally {
        this.javaScanning = false
      }

      return this.javaRuntimes
    },

    async probeJava(path: string): Promise<JavaRuntime | null> {
      if (!path.trim()) return null
      return await call('probe_java', { path })
    },

    async openLauncherDir() {
      if (this.paths) await call('open_path', { path: this.paths.root })
    },

    async checkUpdate(): Promise<AppUpdate | null> {
      if (this.updateBusy) return this.update

      this.updateBusy = 'checking'

      try {
        const next = await check({ timeout: CHECK_TIMEOUT })

        if (found && next && found.version === next.version) {
          await next.close()
        }
        else {
          await found?.close()
          found = next
          this.update = next
            ? { version: next.version, notes: next.body?.trim() || undefined, date: next.date, downloaded: false }
            : null
        }
      }
      finally {
        this.updateBusy = null
      }

      if (this.update && !this.update.downloaded && this.config?.launcher.auto_update) {
        safeRun(() => this.downloadUpdate(), { code: 'UPDATE_FAILED' })
      }

      return this.update
    },

    async downloadUpdate() {
      const update = found
      if (!update || !this.update || this.update.downloaded || this.updateBusy) return

      this.updateBusy = 'downloading'
      this.updateProgress = { downloaded: 0, total: null }

      try {
        await update.download((event) => {
          if (!this.updateProgress) return
          if (event.event === 'Started') this.updateProgress.total = event.data.contentLength ?? null
          if (event.event === 'Progress') this.updateProgress.downloaded += event.data.chunkLength
        }, { timeout: DOWNLOAD_TIMEOUT })

        if (this.update?.version === update.version) this.update.downloaded = true
      }
      finally {
        this.updateProgress = null
        if (this.updateBusy === 'downloading') this.updateBusy = null
      }
    },

    async installUpdate() {
      if (!this.update || this.updateBusy === 'installing') return

      const instances = useInstanceStore()
      if (instances.running.length) {
        throw new LauncherError('CONFLICT', { text: { key: 'error.reason.instance.close_game_first' } })
      }
      if (instances.installs.length) {
        throw new LauncherError('CONFLICT', { text: { key: 'error.reason.instance.wait_for_install' } })
      }

      if (!this.update.downloaded) await this.downloadUpdate()
      if (!found || !this.update.downloaded) return

      trackEvent('app_update_started', { version: this.update.version })

      this.updateBusy = 'installing'

      try {
        await found.install()
        await relaunch()
      }
      finally {
        if (this.updateBusy === 'installing') this.updateBusy = null
      }
    },
  },
})
