<script setup lang="ts">
import type { AppConfig, CustomTheme } from '~/types/app'

const config = defineModel<AppConfig>({ required: true })

const { t } = useI18n()
const toast = useAppToast()
const resolved = useTheme()

const launcher = computed(() => config.value.launcher)
const themes = computed(() => launcher.value.themes)

const editing = computed(() => themes.value.find(theme => theme.id === launcher.value.theme) ?? null)

const pairItems = computed(() => [
  ...THEMES.map(theme => ({ label: t(theme.labelKey), value: theme.value })),
  ...themes.value.map(theme => ({ label: themeName(theme.id, themes.value), value: theme.id })),
])

function addTheme(theme: CustomTheme) {
  launcher.value.themes = [...themes.value, theme]
  launcher.value.theme = theme.id
}

const source = computed(() => (launcher.value.theme === SYSTEM_THEME ? resolved.value : launcher.value.theme))

function duplicate(id: string) {
  const own = themes.value.find(theme => theme.id === id)

  addTheme({
    id: newThemeId(),
    name: t('theme.custom.copy_name', { name: themeName(id, themes.value) }),
    classic: own?.classic ?? id === 'classic',
    colors: own ? { ...own.colors } : readPalette(id),
  })
}

async function share(theme: CustomTheme) {
  const code = await safeRun(() => call('encode_theme', { theme }), { context: { action: t('theme_editor.share') } })
  if (!code) return

  if (await copyToClipboard(code)) {
    toast.add({ title: t('theme_editor.copied'), color: 'success', icon: 'i-lucide-clipboard-check' })
  }
  else {
    toast.add({ title: t('common.copy_failed'), color: 'error', icon: 'i-lucide-clipboard-x' })
  }
}

const removeTarget = ref<CustomTheme | null>(null)

function confirmRemove() {
  const target = removeTarget.value
  if (!target) return

  removeTarget.value = null

  launcher.value.themes = themes.value.filter(theme => theme.id !== target.id)
  if (launcher.value.theme === target.id) launcher.value.theme = DEFAULT_THEME
  if (launcher.value.theme_light === target.id) launcher.value.theme_light = DEFAULT_LIGHT_THEME
  if (launcher.value.theme_dark === target.id) launcher.value.theme_dark = DEFAULT_THEME
}

const importOpen = ref(false)
const importCode = ref('')
const importing = ref(false)

async function importTheme() {
  const code = importCode.value.trim()
  if (!code || importing.value) return

  importing.value = true
  const result = await attempt(() => call('decode_theme', { code }), { context: { action: t('theme_import.action') } })
  importing.value = false

  if (!result.ok) return

  const theme = result.value
  addTheme(theme)

  importOpen.value = false
  importCode.value = ''
  toast.add({ title: t('theme_import.added', { name: themeName(theme.id, themes.value) }), color: 'success', icon: 'i-lucide-palette' })
}
</script>

<template>
  <div class="space-y-6">
    <KitField :label="$t('settings.launcher.theme')">
      <KitThemePicker
        v-model="launcher.theme"
        :custom="themes"
        :light="launcher.theme_light"
        :dark="launcher.theme_dark"
      />

      <div class="mt-3 -ml-2.5 flex gap-1">
        <Button
          variant="ghost"
          size="sm"
          icon="i-lucide-copy"
          @click="duplicate(source)"
        >
          {{ $t('theme.custom.duplicate', { name: themeName(source, themes) }) }}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          icon="i-lucide-clipboard-paste"
          @click="importOpen = true"
        >
          {{ $t('theme.custom.import') }}
        </Button>
      </div>
    </KitField>

    <KitRows v-if="launcher.theme === SYSTEM_THEME">
      <KitRow
        :label="$t('settings.launcher.system_light.label')"
      >
        <KitSelect
          v-model="launcher.theme_light"
          :items="pairItems"
          class="w-50"
        />
      </KitRow>
      <KitRow
        :label="$t('settings.launcher.system_dark.label')"
      >
        <KitSelect
          v-model="launcher.theme_dark"
          :items="pairItems"
          class="w-50"
        />
      </KitRow>
    </KitRows>

    <SettingsThemeEditor
      v-if="editing"
      :key="editing.id"
      :model-value="editing"
      @share="share(editing)"
      @remove="removeTarget = editing"
    />

    <KitField :label="$t('settings.launcher.accent.label')">
      <KitAccentPicker v-model="launcher.accent" />
    </KitField>

    <KitRows>
      <KitRow :label="$t('settings.launcher.language')">
        <KitLanguageSelect
          v-model="launcher.language"
          class="w-50"
        />
      </KitRow>
      <KitRow
        :label="$t('settings.launcher.compact.label')"
        :description="$t('settings.launcher.compact.hint')"
      >
        <Switch v-model="launcher.compact" />
      </KitRow>
    </KitRows>

    <KitConfirmDialog
      :open="!!removeTarget"
      :title="$t('theme_editor.remove_title', { name: removeTarget ? themeName(removeTarget.id, themes) : '' })"
      :description="$t('theme_editor.remove_text')"
      :confirm-label="$t('theme_editor.remove')"
      @update:open="value => { if (!value) removeTarget = null }"
      @confirm="confirmRemove"
    />

    <Dialog v-model:open="importOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{{ $t('theme_import.title') }}</DialogTitle>
        </DialogHeader>
        <DialogBody>
          <Textarea
            v-model="importCode"
            rows="5"
            spellcheck="false"
            placeholder="cast-theme:…"
            :aria-label="$t('theme_import.title')"
            class="font-mono text-caption break-all"
            autofocus
          />
        </DialogBody>
        <DialogFooter>
          <DialogClose as-child>
            <Button variant="ghost">
              {{ $t('common.cancel') }}
            </Button>
          </DialogClose>
          <Button
            variant="primary"
            icon="i-lucide-plus"
            :loading="importing"
            :disabled="!importCode.trim()"
            @click="importTheme"
          >
            {{ $t('theme_import.add') }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
