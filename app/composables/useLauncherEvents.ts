let started: Promise<void> | null = null

export function useLauncherEvents(): Promise<void> {
  if (started) return started

  started = connect().catch((error) => {
    started = null
    throw error
  })

  return started
}

async function connect() {
  const appStore = useAppStore()
  const instanceStore = useInstanceStore()
  const accountStore = useAccountStore()
  const importStore = useImportStore()
  const castStore = useCastStore()

  const bootstrap = await call('bootstrap')

  setTelemetryEnabled(bootstrap.config.launcher.telemetry)

  appStore.applyBootstrap(bootstrap.config, bootstrap.paths)
  accountStore.applyBootstrap(bootstrap.accounts)
  instanceStore.applyBootstrap(bootstrap.instances, bootstrap.installs, bootstrap.running)
  importStore.applyBootstrap(bootstrap.import)

  await onLauncherEvent((event) => {
    switch (event.type) {
      case 'install':
        instanceStore.applyInstall(event)
        break
      case 'import':
        importStore.applyProgress(event)
        break
      case 'importFinished':
        importStore.applyReport(event.report)
        break
      case 'instances':
        instanceStore.applyInstances(event.instances)
        break
      case 'gameStarted':
        instanceStore.applyGameStarted(event.game)
        break
      case 'gameStatus':
        instanceStore.applyGameStatus(event.runId, event.status)
        break
      case 'gameExited':
        instanceStore.applyGameExited(event.runId, event.code, event.logTail)
        break
      case 'gameLog':
        instanceStore.applyGameLog(event.instanceId, {
          runId: event.runId,
          line: event.line,
          isError: event.isError,
        })
        break
      case 'launchFailed':
        captureError(event.error, {
          code: 'LAUNCH_FAILED',
          context: { instanceId: event.instanceId, instanceName: event.instanceName },
        })
        break
      case 'castExport':
        castStore.applyExportProgress(event)
        break
      case 'filesOpened':
        safeRun(() => castStore.takeOpened())
        break
    }
  })

  await safeRun(() => castStore.takeOpened())
}
