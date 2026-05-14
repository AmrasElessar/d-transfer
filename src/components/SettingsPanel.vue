<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import { useSettingsStore } from "@/stores/settings";
import { useThemeStore, type ThemePreference } from "@/stores/theme";
import { useLocaleStore } from "@/stores/locale";
import { useSettingsPanel } from "@/composables/useSettingsPanel";
import FieldNumber from "@/components/widgets/FieldNumber.vue";
import FieldToggle from "@/components/widgets/FieldToggle.vue";
import FieldSegmented from "@/components/widgets/FieldSegmented.vue";
import type { ChecksumAlgo, FsyncPolicy } from "@/types";

const { t } = useI18n();
const settingsStore = useSettingsStore();
const themeStore = useThemeStore();
const localeStore = useLocaleStore();
const panel = useSettingsPanel();

const { settings, lastError } = storeToRefs(settingsStore);

const dialogRef = ref<HTMLDivElement | null>(null);
const saving = ref(false);

// Self-managed visibility — parent prop/emit dance kaldırıldı. Panel kendi
// kapanışını composable üzerinden tetikler; AppShell sadece `<SettingsPanel />`
// render eder.
function close() {
  panel.close();
}

// Bandwidth limit'i Mbps biriminde göster (UI dostluğu), backend bps cinsinden.
const bandwidthMbps = computed({
  get: () => {
    const bps = settings.value.bandwidthLimitBps;
    if (bps === null) return 0;
    return Math.round((bps / 1_000_000) * 10) / 10;
  },
  set: (v: number) => {
    void apply({
      bandwidthLimitBps: v > 0 ? Math.round(v * 1_000_000) : null,
    });
  },
});

async function apply(patch: Parameters<typeof settingsStore.apply>[0]) {
  saving.value = true;
  try {
    await settingsStore.apply(patch);
  } finally {
    saving.value = false;
  }
}

async function pickDownloadDir() {
  const selected = await openDialog({ directory: true, multiple: false });
  if (typeof selected === "string") {
    await apply({ defaultDownloadDir: selected });
  }
}

async function clearDownloadDir() {
  await apply({ defaultDownloadDir: null });
}

const themeOptions = computed(() =>
  (["light", "dark", "system"] as const).map((v) => ({
    value: v,
    label: t(`settings.theme.${v}`),
  })),
);

const localeOptions = computed(() =>
  (["tr", "en"] as const).map((v) => ({
    value: v,
    label: t(`settings.locale.${v}`),
  })),
);

const checksumOptions = computed<{ value: ChecksumAlgo; label: string }[]>(() => [
  { value: "none", label: t("settings.values.checksumNone") },
  { value: "sha256", label: t("settings.values.checksumSha256") },
  { value: "xxHash3", label: t("settings.values.checksumXxHash3") },
]);

const fsyncOptions = computed<{ value: FsyncPolicy; label: string }[]>(() => [
  { value: "none", label: t("settings.values.fsyncNone") },
  { value: "dataOnly", label: t("settings.values.fsyncDataOnly") },
  { value: "full", label: t("settings.values.fsyncFull") },
]);

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && panel.isOpen.value) close();
}

onMounted(() => {
  document.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown);
});

watch(
  () => panel.isOpen.value,
  (now) => {
    if (now) dialogRef.value?.focus();
  },
);
</script>

<template>
  <div
    v-if="panel.isOpen.value"
    class="fixed inset-0 z-50 flex items-center justify-center bg-surface-base/70 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    :aria-label="t('settings.title')"
    @click.self="close"
  >
    <div
      ref="dialogRef"
      tabindex="-1"
      class="flex max-h-[85vh] w-[640px] max-w-[92vw] flex-col rounded-md border border-border-default bg-surface-overlay text-fg-default shadow-2xl outline-none"
    >
      <header class="flex shrink-0 items-center justify-between border-b border-border-muted px-4 py-3">
        <div>
          <h2 class="text-sm font-medium">{{ t("settings.title") }}</h2>
          <p class="text-xs text-fg-muted">{{ t("settings.subtitle") }}</p>
        </div>
        <button
          type="button"
          class="rounded-md border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
          :aria-label="t('settings.actions.close')"
          @click="close"
        >
          ×
        </button>
      </header>

      <div class="flex-1 overflow-y-auto px-4 py-3 text-sm">
        <!-- ========= Appearance ========= -->
        <section class="mb-5">
          <h3 class="mb-2 text-xs font-medium uppercase tracking-wider text-fg-muted">
            {{ t("settings.sections.appearance") }}
          </h3>

          <FieldSegmented
            :label="t('settings.theme.label')"
            :options="themeOptions"
            :model-value="themeStore.preference"
            @update="(v: ThemePreference) => themeStore.setPreference(v)"
          />

          <FieldSegmented
            :label="t('settings.locale.label')"
            :options="localeOptions"
            :model-value="localeStore.locale"
            @update="(v: 'tr' | 'en') => localeStore.setLocale(v)"
          />
        </section>

        <!-- ========= Paths ========= -->
        <section class="mb-5">
          <h3 class="mb-2 text-xs font-medium uppercase tracking-wider text-fg-muted">
            {{ t("settings.sections.paths") }}
          </h3>
          <div class="grid grid-cols-[200px_1fr] items-start gap-3">
            <div>
              <div class="text-fg-default">{{ t("settings.fields.defaultDownloadDir") }}</div>
              <div class="mt-1 text-xs text-fg-subtle">{{ t("settings.fields.defaultDownloadDirHelp") }}</div>
            </div>
            <div class="flex flex-col gap-1">
              <div
                class="truncate rounded border border-border-muted bg-surface-base px-2 py-1 font-mono text-xs"
                :class="settings.defaultDownloadDir ? 'text-fg-default' : 'italic text-fg-subtle'"
              >
                {{ settings.defaultDownloadDir ?? "—" }}
              </div>
              <div class="flex gap-2">
                <button
                  type="button"
                  class="rounded border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
                  @click="pickDownloadDir"
                >
                  {{ t("settings.actions.chooseDir") }}
                </button>
                <button
                  type="button"
                  :disabled="!settings.defaultDownloadDir"
                  class="rounded border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default disabled:cursor-not-allowed disabled:opacity-50"
                  @click="clearDownloadDir"
                >
                  {{ t("settings.actions.clearDir") }}
                </button>
              </div>
            </div>
          </div>
        </section>

        <!-- ========= Transfers ========= -->
        <section class="mb-5">
          <h3 class="mb-2 text-xs font-medium uppercase tracking-wider text-fg-muted">
            {{ t("settings.sections.transfers") }}
          </h3>

          <FieldNumber
            :label="t('settings.fields.maxConcurrentTransfers')"
            :help="t('settings.fields.maxConcurrentTransfersHelp')"
            :model-value="settings.maxConcurrentTransfers"
            :min="1"
            :max="32"
            @update="(v) => apply({ maxConcurrentTransfers: v })"
          />

          <FieldNumber
            :label="t('settings.fields.defaultChunkSizeMb')"
            :help="t('settings.fields.defaultChunkSizeMbHelp')"
            :model-value="settings.defaultChunkSizeMb"
            :min="1"
            :max="1024"
            @update="(v) => apply({ defaultChunkSizeMb: v })"
          />

          <FieldNumber
            :label="t('settings.fields.defaultMaxInflightMb')"
            :help="t('settings.fields.defaultMaxInflightMbHelp')"
            :model-value="settings.defaultMaxInflightMb"
            :min="8"
            :max="4096"
            @update="(v) => apply({ defaultMaxInflightMb: v })"
          />

          <FieldNumber
            :label="t('settings.fields.bandwidthLimitMbps')"
            :help="t('settings.fields.bandwidthLimitMbpsHelp')"
            :model-value="bandwidthMbps"
            :min="0"
            :max="100000"
            :step="0.1"
            @update="(v) => (bandwidthMbps = v)"
          />
        </section>

        <!-- ========= Advanced ========= -->
        <section class="mb-5">
          <h3 class="mb-2 text-xs font-medium uppercase tracking-wider text-fg-muted">
            {{ t("settings.sections.advanced") }}
          </h3>

          <FieldSegmented
            :label="t('settings.fields.verifyChecksum')"
            :help="t('settings.fields.verifyChecksumHelp')"
            :options="checksumOptions"
            :model-value="settings.verifyChecksum"
            @update="(v: ChecksumAlgo) => apply({ verifyChecksum: v })"
          />

          <FieldSegmented
            :label="t('settings.fields.fsyncPolicy')"
            :help="t('settings.fields.fsyncPolicyHelp')"
            :options="fsyncOptions"
            :model-value="settings.fsyncPolicy"
            @update="(v: FsyncPolicy) => apply({ fsyncPolicy: v })"
          />
        </section>

        <!-- ========= Privacy & Updates ========= -->
        <section>
          <h3 class="mb-2 text-xs font-medium uppercase tracking-wider text-fg-muted">
            {{ t("settings.sections.privacy") }}
          </h3>

          <FieldToggle
            :label="t('settings.fields.autoUpdate')"
            :help="t('settings.fields.autoUpdateHelp')"
            :model-value="settings.autoUpdate"
            @update="(v) => apply({ autoUpdate: v })"
          />

          <FieldToggle
            :label="t('settings.fields.telemetry')"
            :help="t('settings.fields.telemetryHelp')"
            :model-value="settings.telemetry"
            @update="(v) => apply({ telemetry: v })"
          />
        </section>

        <div v-if="lastError" class="mt-4 rounded border border-status-danger/40 bg-status-danger/10 px-2 py-1 text-xs text-status-danger">
          {{ lastError }}
        </div>
      </div>

      <footer class="flex shrink-0 items-center justify-between border-t border-border-muted px-4 py-2 text-xs text-fg-muted">
        <span>{{ saving ? "…" : "" }}</span>
        <button
          type="button"
          class="rounded border border-border-muted px-2 py-1 text-xs hover:bg-surface-overlay hover:text-fg-default"
          @click="close"
        >
          {{ t("settings.actions.close") }}
        </button>
      </footer>
    </div>
  </div>
</template>
