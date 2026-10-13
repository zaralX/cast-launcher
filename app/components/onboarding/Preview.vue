<script setup lang="ts">
defineProps<{
  compact: boolean
}>()
</script>

<template>
  <div
    class="overflow-hidden rounded-md bg-frame ring-1 ring-line-strong select-none"
    aria-hidden="true"
  >
    <div class="flex h-5 items-center gap-1.5 px-2">
      <span class="size-2 rounded-xs bg-accent" />
      <span class="h-1 w-6 rounded-pill bg-line-strong" />
      <span class="ml-auto flex items-center gap-2">
        <span class="h-px w-1.5 bg-fg-faint" />
        <span class="size-1.5 ring-1 ring-fg-faint ring-inset" />
        <span class="size-1.5 rounded-pill bg-fg-faint" />
      </span>
    </div>

    <div class="flex h-40">
      <div class="flex w-6 shrink-0 flex-col items-center gap-1 pt-0.5 pb-1.5">
        <span class="grid size-4 place-items-center rounded-sm bg-accent/18">
          <span class="size-1.5 rounded-xs bg-accent" />
        </span>
        <span
          v-for="item in 2"
          :key="item"
          class="grid size-4 place-items-center"
        >
          <span class="size-1.5 rounded-xs bg-line-strong" />
        </span>
        <span class="flex-1" />
        <span class="size-3 rounded-xs bg-control" />
      </div>

      <div class="flex min-w-0 flex-1 overflow-hidden rounded-tl-md bg-sheet shadow-sheet">
        <div class="min-w-0 flex-1 p-2">
          <div class="flex items-center gap-1">
            <span class="h-2.5 w-14 rounded-xs bg-control" />
            <span class="h-2.5 w-7 rounded-xs bg-control/60" />
            <span class="flex-1" />
            <span class="h-2.5 w-10 rounded-xs bg-control" />
          </div>

          <Transition
            name="mock"
            mode="out-in"
          >
            <div
              v-if="compact"
              key="compact"
              class="mt-2 grid grid-cols-4 gap-1"
            >
              <span
                v-for="tile in 12"
                :key="tile"
                class="grid aspect-square place-items-center rounded-sm"
                :class="tile === 1 ? 'bg-accent/18' : 'bg-panel'"
              >
                <span
                  class="size-3 rounded-xs"
                  :class="tile === 1 ? 'bg-accent' : 'bg-control'"
                />
              </span>
            </div>

            <div
              v-else
              key="cards"
              class="mt-2 grid grid-cols-2 gap-1"
            >
              <span
                v-for="card in 6"
                :key="card"
                class="flex h-7 items-center gap-1 rounded-sm bg-panel px-1 ring-1 ring-card-line ring-inset"
              >
                <span
                  class="size-4.5 shrink-0 rounded-xs"
                  :class="card === 1 ? 'bg-accent' : 'bg-control'"
                />
                <span class="min-w-0 flex-1 space-y-0.5">
                  <span class="block h-1 w-4/5 rounded-pill bg-fg-faint/50" />
                  <span class="block h-0.75 w-1/2 rounded-pill bg-line-strong" />
                </span>
                <span class="size-2.5 shrink-0 rounded-pill bg-control" />
              </span>
            </div>
          </Transition>
        </div>

        <div
          v-if="compact"
          class="w-14 shrink-0 border-l border-line bg-panel p-1.5"
        >
          <span class="block size-6 rounded-sm bg-accent" />
          <span class="mt-1.5 block h-1 w-4/5 rounded-pill bg-fg-faint/50" />
          <span class="mt-1 block h-0.75 w-1/2 rounded-pill bg-line-strong" />
          <span class="mt-2 block h-2.5 w-full rounded-xs bg-accent" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mock-enter-active,
.mock-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.mock-enter-from {
  opacity: 0;
  transform: translateY(4px);
}

.mock-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
