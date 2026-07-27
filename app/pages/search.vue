<script setup lang="ts">
definePageMeta({
  layout: "main"
});

const modPlatforms = ref([
  {name: "Modrinth", status: "В работе"},
  {name: "CurseForge", status: "В очереди"}
])
</script>

<template>
  <div class="min-h-full w-full px-8 pb-16 pt-10 xl:px-14">
    <header class="border-b border-line pb-8">
      <p class="font-mono text-[10px] uppercase tracking-[0.4em] text-fg-faint">Каталог</p>
      <h1 class="mt-4 font-unbounded text-[clamp(30px,4.2vw,46px)] font-bold leading-[0.92] tracking-[-0.055em] text-fg">
        Поиск сборок<span class="text-acid">.</span>
      </h1>
    </header>

    <!-- Честная заглушка: раздел выключен, но интерфейс объясняет, что здесь будет -->
    <section class="mt-16 grid gap-14 lg:grid-cols-[minmax(0,26rem)_minmax(0,1fr)]">
      <div class="animate-rise">
        <div class="flex items-center gap-3">
          <span class="size-1.5 bg-acid animate-blink"/>
          <span class="font-mono text-[10px] uppercase tracking-[0.28em] text-acid">Раздел в разработке</span>
        </div>
        <p class="mt-6 text-[13px] leading-relaxed text-fg-muted">
          Здесь появится поиск по внешним каталогам модпаков: установка в один клик,
          синхронизация версий загрузчика и обновление уже созданных сборок.
        </p>
      </div>

      <ul class="animate-rise [animation-delay:120ms]">
        <li
            v-for="(platform, i) in modPlatforms"
            :key="platform.name"
            class="group flex items-baseline gap-6 border-t border-line py-6 last:border-b"
        >
          <span class="font-mono text-[10px] leading-none text-fg-faint">
            {{ String(i + 1).padStart(2, "0") }}
          </span>
          <span
              class="font-unbounded text-[18px] font-semibold leading-none tracking-[-0.04em] text-fg-muted transition-colors duration-500 group-hover:text-fg"
          >
            {{ platform.name }}
          </span>
          <span class="h-px flex-1 bg-line"/>
          <span class="font-mono text-[10px] uppercase tracking-[0.16em] text-fg-faint">{{ platform.status }}</span>
        </li>
      </ul>
    </section>
  </div>
</template>
