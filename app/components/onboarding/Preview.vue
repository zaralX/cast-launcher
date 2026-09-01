<script setup lang="ts">
defineProps<{
  compact: boolean
}>()

const ROWS = [
  {name: "Vanilla", meta: "1.21.4", width: "w-16"},
  {name: "Fabric", meta: "1.20.1", width: "w-20"},
  {name: "Create", meta: "1.20.1", width: "w-14"},
  {name: "TFG", meta: "1.19.2", width: "w-24"},
  {name: "Skyblock", meta: "1.8.9", width: "w-16"}
]
</script>

<template>
  <div class="relative select-none overflow-hidden border border-line bg-ink-900">
    <div class="flex h-6 items-center gap-2 border-b border-line bg-ink-800 px-2.5">
      <span class="size-1.5 bg-acid"/>
      <span class="h-1 w-10 bg-line-strong"/>
      <span class="ml-auto flex items-center gap-1.5">
        <span class="h-px w-2 bg-fg-faint/60"/>
        <span class="size-1.5 border border-fg-faint/60"/>
        <span class="size-1.5 rotate-45 border-r border-t border-fg-faint/60"/>
      </span>
    </div>

    <div class="flex h-[9.5rem]">
      <div class="flex w-7 shrink-0 flex-col items-center gap-2 border-r border-line bg-ink-800/60 py-2.5">
        <span class="relative grid size-4 place-items-center">
          <span class="absolute -left-2.5 h-3 w-px bg-acid"/>
          <span class="size-2.5 border border-acid"/>
        </span>
        <span v-for="i in 3" :key="i" class="size-2.5 border border-fg-faint/40"/>
      </div>

      <div class="min-w-0 flex-1 p-3">
        <Transition name="mock" mode="out-in">
          <div v-if="compact" key="compact" class="space-y-[3px]">
            <div
                v-for="(row, i) in ROWS"
                :key="row.name"
                class="relative flex items-center gap-2 border border-line/70 bg-ink-800/60 px-2 py-[5px]"
                :class="i === 0 ? 'border-acid/40' : ''"
            >
              <span v-if="i === 0" class="absolute inset-y-0 left-0 w-[2px] bg-acid"/>
              <span class="size-2.5 shrink-0" :class="i === 0 ? 'bg-acid' : 'bg-line-strong'"/>
              <span class="font-mono text-[6px] uppercase tracking-[0.18em] text-fg-muted">{{ row.name }}</span>
              <span class="ml-auto font-mono text-[6px] text-fg-faint">{{ row.meta }}</span>
            </div>
          </div>

          <div v-else key="cards" class="grid grid-cols-2 gap-2">
            <div
                v-for="(row, i) in ROWS.slice(0, 4)"
                :key="row.name"
                class="border border-line/70 bg-ink-800/60 p-2"
                :class="i === 0 ? 'border-acid/40' : ''"
            >
              <span class="block size-6" :class="i === 0 ? 'bg-acid' : 'bg-line-strong'"/>
              <span class="mt-2 block font-mono text-[6px] uppercase tracking-[0.18em] text-fg-muted">
                {{ row.name }}
              </span>
              <span class="mt-1 block h-px bg-line" :class="row.width"/>
              <span
                  class="mt-2 block h-2 w-full"
                  :class="i === 0 ? 'bg-acid/70' : 'bg-line'"
              />
            </div>
          </div>
        </Transition>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mock-enter-active,
.mock-leave-active {
  transition: opacity 0.28s cubic-bezier(0.16, 1, 0.3, 1), transform 0.28s cubic-bezier(0.16, 1, 0.3, 1);
}

.mock-enter-from {
  opacity: 0;
  transform: translateY(6px) scale(0.98);
}

.mock-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.98);
}
</style>
