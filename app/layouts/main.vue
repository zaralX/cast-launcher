<template>
  <div class="flex h-screen w-full flex-col overflow-hidden bg-frame text-fg">
    <AppTitleBar />

    <div class="flex min-h-0 flex-1">
      <AppNavRail />

      <main class="relative min-w-0 flex-1 overflow-hidden rounded-tl-lg bg-sheet shadow-sheet">
        <div
          data-main-scroll
          class="h-full w-full overflow-x-hidden overflow-y-auto"
        >
          <div class="relative flex min-h-full flex-col">
            <slot />
          </div>
        </div>
      </main>
    </div>
  </div>
</template>
