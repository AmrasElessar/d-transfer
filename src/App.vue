<script setup lang="ts">
import { onMounted, watchEffect } from "vue";
import { storeToRefs } from "pinia";
import { useThemeStore } from "@/stores/theme";
import { useLocaleStore } from "@/stores/locale";
import { useSettingsStore } from "@/stores/settings";
import { useEngineEvents } from "@/composables/useEngineEvents";
import AppShell from "@/components/AppShell.vue";

const themeStore = useThemeStore();
const localeStore = useLocaleStore();
const settingsStore = useSettingsStore();

// Singleton subscriber for the engine-event broadcast bridged from Rust.
useEngineEvents();

const { resolvedTheme } = storeToRefs(themeStore);
const { locale } = storeToRefs(localeStore);

watchEffect(() => {
  const root = document.documentElement;
  root.classList.toggle("theme-dark", resolvedTheme.value === "dark");
  root.classList.toggle("theme-light", resolvedTheme.value === "light");
  root.lang = locale.value;
});

onMounted(() => {
  themeStore.init();
  localeStore.init();
  void settingsStore.loadOnce();
});
</script>

<template>
  <AppShell />
</template>
