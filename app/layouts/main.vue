<template>
  <div class="flex h-screen w-full flex-col overflow-hidden bg-ink-900 text-fg">
    <AppTitleBar />

    <div class="flex min-h-0 flex-1">
      <AppNavRail />

      <main class="relative min-w-0 flex-1">
        <div
          class="pointer-events-none absolute -top-32 -left-32 z-10 size-96 rounded-full bg-acid/[0.05] blur-[130px]"
          aria-hidden="true"
        />
        <div class="relative h-[calc(100vh-2.75rem)] w-full overflow-x-hidden overflow-y-auto">
          <div class="relative flex flex-col">
            <slot />
          </div>
        </div>
      </main>
    </div>
  </div>
</template>
