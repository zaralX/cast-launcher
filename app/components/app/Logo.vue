<template>
  <div class="pointer-events-none flex items-center select-none">
    <span class="grid w-14 shrink-0 place-items-center">
      <img
        src="/logo.svg"
        class="size-6.5"
        alt=""
      >
    </span>
    <p class="flex items-baseline gap-2 pr-6">
      <span class="font-unbounded text-title font-semibold tracking-heading classic:uppercase">Cast</span>
      <slot />
    </p>
  </div>
</template>
