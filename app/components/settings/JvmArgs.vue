<script setup lang="ts">
const args = defineModel<string>({ required: true })

const MEMORY_FLAG = /(?:^|[\s"'])-(?:Xm[sx]|XX:(?:Max|Min|Initial)HeapSize)/

const setsMemory = computed(() => MEMORY_FLAG.test(args.value))
</script>

<template>
  <div class="space-y-2">
    <KitField
      :label="$t('settings.java.jvm_args.label')"
      :hint="$t('settings.java.jvm_args.hint')"
      for="jvm-args"
    >
      <Textarea
        id="jvm-args"
        v-model="args"
        :rows="2"
        autoresize
        spellcheck="false"
        autocomplete="off"
        placeholder="-XX:+UseG1GC -Dfile.encoding=UTF-8"
        class="font-mono text-caption break-all"
      />
    </KitField>

    <KitStatus
      v-if="setsMemory"
      tone="warning"
    >
      {{ $t('settings.java.jvm_args.memory_warning') }}
    </KitStatus>
  </div>
</template>
