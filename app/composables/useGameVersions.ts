import type { InstanceType } from '~/types/instance'
import type { NeoForgeRelease } from '~/types/backend'
import type { LauncherError } from '~/utils/error'

export function useGameVersions() {
  const { t } = useI18n()

  const loading = ref(true)
  const loadError = ref<LauncherError | null>(null)

  const minecraft = ref<string[]>([])
  const latestRelease = ref('')
  const fabric = ref<string[]>([])
  const forge = ref<string[]>([])
  const neoforge = ref<NeoForgeRelease[]>([])

  async function load() {
    loading.value = true
    loadError.value = null

    try {
      const [manifest, fabricVersions, forgeVersions, neoforgeVersions] = await Promise.all([
        call('list_minecraft_versions'),
        call('list_fabric_versions'),
        call('list_forge_versions'),
        call('list_neoforge_versions'),
      ])

      minecraft.value = manifest.versions
        .filter(version => version.type === 'release')
        .map(version => version.id)
      latestRelease.value = manifest.latest.release ?? minecraft.value[0] ?? ''
      fabric.value = fabricVersions
      forge.value = forgeVersions
      neoforge.value = neoforgeVersions
    }
    catch (e) {
      loadError.value = captureError(e, { code: 'NETWORK', context: { action: t('create.versions_action') } })
    }
    finally {
      loading.value = false
    }
  }

  function loaderVersions(type: InstanceType, minecraftVersion: string): string[] {
    if (type === 'fabric') return fabric.value
    if (type === 'forge') return forge.value.filter(version => version.startsWith(`${minecraftVersion}-`))
    if (type === 'neoforge') {
      return neoforge.value
        .filter(release => release.minecraftVersion === minecraftVersion)
        .map(release => release.version)
    }
    return []
  }

  onMounted(load)

  return { loading, loadError, load, minecraft, latestRelease, loaderVersions }
}
