<script setup lang="ts">
import type { CustomTheme } from '~/types/app'

const theme = defineModel<CustomTheme>({ required: true })

const emit = defineEmits<{
  share: []
  remove: []
}>()
</script>

<template>
  <KitPanel
    :title="$t('theme_editor.title')"
    icon="i-lucide-palette"
  >
    <template #action>
      <Button
        variant="ghost"
        size="sm"
        icon="i-lucide-share-2"
        @click="emit('share')"
      >
        {{ $t('theme_editor.share') }}
      </Button>
      <Button
        variant="danger-ghost"
        size="sm"
        icon="i-lucide-trash-2"
        class="-mr-1.5"
        @click="emit('remove')"
      >
        {{ $t('theme_editor.remove') }}
      </Button>
    </template>

    <div class="grid grid-cols-2 items-end gap-x-6">
      <KitField
        :label="$t('theme_editor.name')"
        for="theme-name"
      >
        <Input
          id="theme-name"
          v-model="theme.name"
          maxlength="40"
          :placeholder="$t('theme.custom.untitled')"
        />
      </KitField>

      <KitRow
        :label="$t('theme_editor.classic')"
        :description="$t('theme_editor.classic_hint')"
      >
        <Switch v-model="theme.classic" />
      </KitRow>
    </div>

    <section
      v-for="group in THEME_TOKEN_GROUPS"
      :key="group.labelKey"
      class="mt-5 border-t border-line pt-4"
    >
      <Label class="mb-1.5">
        {{ $t(group.labelKey) }}
      </Label>
      <div class="grid grid-cols-2 gap-x-6 xl:grid-cols-3">
        <KitColorField
          v-for="item in group.tokens"
          :key="item.token"
          v-model="theme.colors[item.token]"
          :label="$t(item.labelKey)"
          :alpha="item.alpha"
        />
      </div>
    </section>
  </KitPanel>
</template>
