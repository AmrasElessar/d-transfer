<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useThemeStore } from "@/stores/theme";
import { useLocaleStore } from "@/stores/locale";
import { useEngine } from "@/composables/useEngine";
import AdapterProbe from "@/components/AdapterProbe.vue";
import TransferProbe from "@/components/TransferProbe.vue";
import { useSettingsPanel } from "@/composables/useSettingsPanel";

const settingsPanel = useSettingsPanel();

const { t } = useI18n();
const themeStore = useThemeStore();
const localeStore = useLocaleStore();
const { running, subscriberCount } = useEngine();

const engineBadgeText = computed(() =>
  running.value
    ? `${t("engine.running")}: ${subscriberCount.value} ${t("engine.subscribers")}`
    : t("engine.stopped"),
);

function cycleTheme() {
  const next =
    themeStore.preference === "light"
      ? "dark"
      : themeStore.preference === "dark"
        ? "system"
        : "light";
  themeStore.setPreference(next);
}

function toggleLocale() {
  localeStore.setLocale(localeStore.locale === "tr" ? "en" : "tr");
}
</script>

<template>
  <header
    data-tauri-drag-region
    class="flex h-10 shrink-0 items-center justify-between border-b border-border-muted bg-surface-raised px-3 text-sm"
  >
    <div class="flex items-center gap-2">
      <span
        class="inline-block size-2.5 rounded-sm bg-accent-default"
        aria-hidden="true"
      />
      <span class="font-medium tracking-tight">{{ t("app.name") }}</span>
      <span class="text-fg-muted">·</span>
      <span class="text-xs text-fg-muted">{{ t("app.tagline") }}</span>
    </div>

    <div class="flex items-center gap-2">
      <div
        class="flex items-center gap-1.5 rounded-md border border-border-muted px-2 py-1 text-xs text-fg-muted"
        :aria-label="engineBadgeText"
        aria-live="polite"
      >
        <span
          class="inline-block size-2 rounded-full"
          :class="running ? 'bg-status-success' : 'bg-status-danger'"
          aria-hidden="true"
        />
        <span v-if="running">
          {{ t("engine.running") }}:
          <span class="font-mono">{{ subscriberCount }}</span>
        </span>
        <span v-else>{{ t("engine.stopped") }}</span>
      </div>

      <AdapterProbe />
      <TransferProbe />
      <button
        type="button"
        class="rounded-md border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
        :aria-label="t('settings.actions.open')"
        @click="settingsPanel.open()"
      >
        {{ t("settings.actions.open") }}
      </button>

      <button
        type="button"
        class="rounded-md border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
        :aria-label="t('settings.locale.label')"
        @click="toggleLocale"
      >
        {{ localeStore.locale.toUpperCase() }}
      </button>
      <button
        type="button"
        class="rounded-md border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
        :aria-label="t('settings.theme.label')"
        @click="cycleTheme"
      >
        {{ t(`settings.theme.${themeStore.preference}`) }}
      </button>
    </div>
  </header>
</template>
