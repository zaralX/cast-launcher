<template>
  <div class="pointer-events-none flex items-center gap-3 pr-6 pl-3 select-none">
    <img
      src="/logo.svg"
      class="size-8"
      alt=""
    >
    <p class="font-unbounded text-title font-semibold leading-none tracking-[-0.05em]">
      CAST<span class="text-acid">.</span>
    </p>
    <slot />
  </div>
</template>
