<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { storeToRefs } from "pinia";
import { RecycleScroller } from "vue-virtual-scroller";
import "vue-virtual-scroller/dist/vue-virtual-scroller.css";
import { useLocalBrowser, type SelectionMode } from "@/composables/useLocalBrowser";
import { useProfilesStore } from "@/stores/profiles";
import { useTransferTargetStore } from "@/stores/transferTargets";
import { formatBytes, formatModifiedTime } from "@/utils/format";
import type { EnqueueTransferResponse, LocalEntry } from "@/types";

const { t } = useI18n();

const browser = useLocalBrowser();
const profilesStore = useProfilesStore();
const { activeProfile } = storeToRefs(profilesStore);
const targets = useTransferTargetStore();

// Pane'in cwd'sini transferTarget store'una aynalayalım — RemotePane'in
// Download butonu hedef yerel klasörü buradan okur.
watch(
  () => browser.cwd.value,
  (v) => targets.setLocalCwd(v),
  { immediate: true },
);

const uploadError = ref<string | null>(null);
const uploading = ref(false);

// Sadece dosya seçimleri yüklenebilir (klasör recursive upload henüz yok).
const selectedFiles = computed<LocalEntry[]>(() =>
  browser.entries.value.filter(
    (e) => browser.selection.value.has(e.path) && e.kind === "file",
  ),
);

const canUpload = computed(
  () =>
    !!activeProfile.value &&
    selectedFiles.value.length > 0 &&
    !uploading.value,
);

function joinRemote(base: string, name: string): string {
  if (!base || base === "" || base === ".") return name;
  if (base.endsWith("/")) return base + name;
  return `${base}/${name}`;
}

async function uploadSelection() {
  if (!activeProfile.value) {
    uploadError.value = t("transfer.errors.noActiveProfile");
    return;
  }
  if (selectedFiles.value.length === 0) return;
  uploadError.value = null;
  uploading.value = true;
  try {
    for (const entry of selectedFiles.value) {
      const remote = joinRemote(targets.remoteCwd, entry.name);
      await invoke<EnqueueTransferResponse>("enqueue_upload", {
        request: {
          profileId: activeProfile.value.id,
          localPath: entry.path,
          remotePath: remote,
          bytesTotal: entry.size ?? 0,
        },
      });
    }
    // Seçimi koruyoruz — kullanıcı aynı dosyaları başka yere de gönderebilir.
  } catch (err) {
    uploadError.value = err instanceof Error ? err.message : String(err);
  } finally {
    uploading.value = false;
  }
}

// Drive dropdown panel görünürlüğü — native <select> yasak (Bölüm 19.5),
// custom button + listbox kullanıyoruz.
const driveDropdownOpen = ref(false);
const drives = ref<string[]>([]);

function isTauriEnv(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function toggleDriveDropdown() {
  if (driveDropdownOpen.value) {
    driveDropdownOpen.value = false;
    return;
  }
  if (!isTauriEnv()) return;
  try {
    drives.value = await invoke<string[]>("list_local_drives");
    driveDropdownOpen.value = true;
  } catch {
    // List drives best-effort; hata göstermeye değmez (drive listesi prefetch).
    driveDropdownOpen.value = false;
  }
}

async function pickDrive(drive: string) {
  driveDropdownOpen.value = false;
  await browser.navigate(drive);
}

async function toggleHidden() {
  browser.includeHidden.value = !browser.includeHidden.value;
  await browser.refresh();
}

/**
 * Breadcrumb path bileşenleri. Windows: `C:\Users\engin` → ["C:", "Users", "engin"]
 * POSIX: `/home/engin` → ["/", "home", "engin"]
 *
 * Her item tıklanabilir: kümülatif path'e navigate.
 */
const breadcrumb = computed<{ label: string; target: string }[]>(() => {
  const p = browser.cwd.value;
  if (!p) return [];
  // Windows canonical bazen `\\?\C:\...` prefix'i alır — UI'da gizle ama
  // navigation target'inde tut (canonicalize round-trip için).
  const stripped = p.startsWith("\\\\?\\") ? p.slice(4) : p;
  // Drive prefix detection — Windows
  const driveMatch = stripped.match(/^([A-Za-z]):[\\/]?/);
  if (driveMatch) {
    const drive = driveMatch[1].toUpperCase() + ":";
    const rest = stripped.slice(driveMatch[0].length);
    const parts = rest.split(/[\\/]+/).filter(Boolean);
    const items: { label: string; target: string }[] = [
      { label: drive, target: `${drive}\\` },
    ];
    let acc = `${drive}\\`;
    for (const part of parts) {
      acc = acc.endsWith("\\") ? acc + part : `${acc}\\${part}`;
      items.push({ label: part, target: acc });
    }
    return items;
  }
  // POSIX path
  if (stripped.startsWith("/")) {
    const parts = stripped.split("/").filter(Boolean);
    const items: { label: string; target: string }[] = [
      { label: "/", target: "/" },
    ];
    let acc = "";
    for (const part of parts) {
      acc = `${acc}/${part}`;
      items.push({ label: part, target: acc });
    }
    return items;
  }
  // Fallback — relative path görünmemeli ama yine de bir şey gösterelim.
  return [{ label: stripped, target: stripped }];
});

async function navigateCrumb(target: string) {
  await browser.navigate(target);
}

// Click handler — modifier'a göre selection mode + double-click navigate.
let lastClickPath: string | null = null;
let lastClickAt = 0;
const DOUBLE_CLICK_MS = 350;

function onEntryClick(entry: LocalEntry, event: MouseEvent) {
  const now = Date.now();
  // Tek seferde double-click detection. Native dblclick handler 2 ayrı tıklama
  // emit ediyor; manuel zamanlama ile sadece dir'lerde navigate'e izin veriyoruz.
  if (
    lastClickPath === entry.path &&
    now - lastClickAt < DOUBLE_CLICK_MS &&
    entry.kind === "directory"
  ) {
    lastClickPath = null;
    lastClickAt = 0;
    void browser.navigate(entry.path);
    return;
  }
  lastClickPath = entry.path;
  lastClickAt = now;

  let mode: SelectionMode = "single";
  if (event.shiftKey) mode = "shift";
  else if (event.ctrlKey || event.metaKey) mode = "ctrl";
  browser.toggleSelect(entry.path, mode);
}

function entryGlyph(entry: LocalEntry): string {
  switch (entry.kind) {
    case "directory":
      return "▸";
    case "symlink":
      return "↪";
    case "file":
      return " ";
    default:
      return "?";
  }
}

function isSelected(entry: LocalEntry): boolean {
  return browser.selection.value.has(entry.path);
}

// Vue-virtual-scroller perf: >200 entry için aktive et. Küçük dizinlerde
// virtual scroll overhead'i CSS layout'tan daha pahalı.
const VIRTUAL_THRESHOLD = 200;
const useVirtual = computed(() => browser.entries.value.length > VIRTUAL_THRESHOLD);

// RecycleScroller item-size sabit gerektiriyor — satır yüksekliği px.
const ROW_HEIGHT = 24;
</script>

<template>
  <div
    class="flex min-h-0 flex-col rounded-sm border border-border-muted bg-surface-raised"
    :aria-label="t('panes.local')"
  >
    <header
      class="flex shrink-0 flex-col gap-1 border-b border-border-muted px-2 py-1.5"
    >
      <div class="flex items-center gap-2">
        <span
          class="text-xs font-medium uppercase tracking-wider text-fg-muted"
        >
          {{ t("panes.local") }}
        </span>
        <div class="flex-1" />
        <!-- Toolbar buttons -->
        <button
          type="button"
          class="rounded border border-transparent px-1.5 py-0.5 text-xs text-fg-muted hover:border-border-muted hover:bg-surface-overlay hover:text-fg-default"
          :title="t('localBrowser.actions.refresh')"
          :aria-label="t('localBrowser.actions.refresh')"
          @click="browser.refresh()"
        >
          ↻
        </button>
        <button
          type="button"
          class="rounded border border-transparent px-1.5 py-0.5 text-xs text-fg-muted hover:border-border-muted hover:bg-surface-overlay hover:text-fg-default"
          :title="t('localBrowser.actions.up')"
          :aria-label="t('localBrowser.actions.up')"
          :disabled="!browser.parent.value"
          :class="{ 'cursor-not-allowed opacity-40': !browser.parent.value }"
          @click="browser.up()"
        >
          ↑
        </button>
        <button
          type="button"
          class="rounded border border-transparent px-1.5 py-0.5 text-xs text-fg-muted hover:border-border-muted hover:bg-surface-overlay hover:text-fg-default"
          :title="t('localBrowser.actions.home')"
          :aria-label="t('localBrowser.actions.home')"
          @click="browser.home()"
        >
          ⌂
        </button>
        <button
          type="button"
          class="rounded border border-transparent px-1.5 py-0.5 text-xs hover:border-border-muted hover:bg-surface-overlay"
          :title="t('localBrowser.actions.toggleHidden')"
          :aria-label="t('localBrowser.actions.toggleHidden')"
          :aria-pressed="browser.includeHidden.value"
          :class="
            browser.includeHidden.value
              ? 'text-accent-default'
              : 'text-fg-muted hover:text-fg-default'
          "
          @click="toggleHidden()"
        >
          ⊙
        </button>
        <!-- Upload selection → remote pane'in cwd'sine -->
        <button
          type="button"
          class="rounded border border-border-muted bg-accent-default/15 px-2 py-0.5 text-xs font-medium text-accent-default hover:bg-accent-default/25 disabled:cursor-not-allowed disabled:opacity-40"
          :title="t('transfer.actions.upload')"
          :aria-label="t('transfer.actions.upload')"
          :disabled="!canUpload"
          @click="uploadSelection()"
        >
          ↑ {{ selectedFiles.length > 0 ? `(${selectedFiles.length})` : "" }}
        </button>
        <!-- Drive selector (Windows: A-Z; POSIX: tek kök) -->
        <div class="relative">
          <button
            type="button"
            class="rounded border border-border-muted px-1.5 py-0.5 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
            :aria-haspopup="true"
            :aria-expanded="driveDropdownOpen"
            @click="toggleDriveDropdown()"
          >
            ▾
          </button>
          <div
            v-if="driveDropdownOpen"
            class="absolute right-0 top-full z-10 mt-1 min-w-[8rem] rounded-md border border-border-default bg-surface-overlay py-1 shadow-lg"
            role="listbox"
          >
            <button
              v-for="d in drives"
              :key="d"
              type="button"
              class="block w-full px-3 py-1 text-left text-xs text-fg-default hover:bg-surface-raised"
              role="option"
              @click="pickDrive(d)"
            >
              {{ d }}
            </button>
            <div
              v-if="drives.length === 0"
              class="px-3 py-1 text-xs text-fg-subtle"
            >
              —
            </div>
          </div>
        </div>
      </div>

      <!-- Breadcrumb -->
      <nav
        class="flex items-center gap-0.5 overflow-x-auto whitespace-nowrap font-mono text-xs text-fg-muted"
        :aria-label="t('panes.local')"
      >
        <template v-for="(crumb, i) in breadcrumb" :key="crumb.target">
          <span v-if="i > 0" class="text-fg-subtle">›</span>
          <button
            type="button"
            class="rounded px-1 py-0.5 hover:bg-surface-overlay hover:text-fg-default"
            @click="navigateCrumb(crumb.target)"
          >
            {{ crumb.label }}
          </button>
        </template>
        <span v-if="breadcrumb.length === 0 && browser.loading.value" class="px-1">
          …
        </span>
      </nav>
    </header>

    <!-- Error banner -->
    <div
      v-if="browser.error.value"
      class="shrink-0 border-b border-border-muted bg-status-danger/10 px-3 py-1.5 text-xs text-status-danger"
      role="alert"
    >
      {{ t("localBrowser.errorPrefix") }}: {{ browser.error.value }}
    </div>
    <div
      v-if="uploadError"
      class="shrink-0 border-b border-status-danger/40 bg-status-danger/10 px-3 py-1.5 text-xs text-status-danger"
      role="alert"
    >
      {{ uploadError }}
    </div>

    <!-- Body -->
    <div class="flex min-h-0 flex-1 flex-col">
      <!-- Column headers -->
      <div
        class="grid shrink-0 grid-cols-[1.5rem_1fr_6rem_8rem] gap-2 border-b border-border-muted px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-fg-subtle"
      >
        <span />
        <span>{{ t("localBrowser.columns.name") }}</span>
        <span class="text-right">{{ t("localBrowser.columns.size") }}</span>
        <span class="text-right">{{ t("localBrowser.columns.modified") }}</span>
      </div>

      <!-- Empty -->
      <div
        v-if="
          !browser.loading.value &&
          !browser.error.value &&
          browser.entries.value.length === 0 &&
          !browser.parent.value
        "
        class="flex flex-1 items-center justify-center text-xs text-fg-subtle"
      >
        {{ t("localBrowser.emptyDir") }}
      </div>

      <!-- Rows: parent row + entries -->
      <div v-else class="min-h-0 flex-1 overflow-hidden font-mono text-xs">
        <!-- Parent "../" — fixed row at top -->
        <div
          v-if="browser.parent.value"
          class="grid cursor-pointer grid-cols-[1.5rem_1fr_6rem_8rem] items-center gap-2 px-2 py-1 hover:bg-surface-overlay"
          :style="{ height: ROW_HEIGHT + 'px' }"
          @click="browser.up()"
        >
          <span class="text-fg-muted">↑</span>
          <span class="truncate text-fg-default">..</span>
          <span class="text-right text-fg-subtle">—</span>
          <span class="text-right text-fg-subtle">—</span>
        </div>

        <!-- Virtual scroller for large dirs -->
        <RecycleScroller
          v-if="useVirtual"
          class="h-full"
          :items="browser.entries.value"
          :item-size="ROW_HEIGHT"
          key-field="path"
          v-slot="{ item }"
        >
          <div
            class="grid cursor-pointer grid-cols-[1.5rem_1fr_6rem_8rem] items-center gap-2 px-2 py-1"
            :style="{ height: ROW_HEIGHT + 'px' }"
            :class="[
              isSelected(item) ? 'bg-accent-subtle text-fg-default' : 'hover:bg-surface-overlay',
              item.isHidden ? 'text-fg-subtle' : '',
            ]"
            @click="onEntryClick(item, $event)"
          >
            <span
              :class="
                item.kind === 'directory'
                  ? 'text-accent-default'
                  : 'text-fg-subtle'
              "
            >
              {{ entryGlyph(item) }}
            </span>
            <span class="truncate">{{ item.name }}</span>
            <span class="text-right text-fg-muted">
              {{ item.kind === "directory" ? "—" : formatBytes(item.size) }}
            </span>
            <span class="text-right text-fg-muted">
              {{ formatModifiedTime(item.modifiedUnixMs) }}
            </span>
          </div>
        </RecycleScroller>

        <!-- Simple list for small dirs — virtual overhead'i bypass -->
        <div v-else class="h-full overflow-auto">
          <div
            v-for="item in browser.entries.value"
            :key="item.path"
            class="grid cursor-pointer grid-cols-[1.5rem_1fr_6rem_8rem] items-center gap-2 px-2 py-1"
            :style="{ height: ROW_HEIGHT + 'px' }"
            :class="[
              isSelected(item) ? 'bg-accent-subtle text-fg-default' : 'hover:bg-surface-overlay',
              item.isHidden ? 'text-fg-subtle' : '',
            ]"
            @click="onEntryClick(item, $event)"
          >
            <span
              :class="
                item.kind === 'directory'
                  ? 'text-accent-default'
                  : 'text-fg-subtle'
              "
            >
              {{ entryGlyph(item) }}
            </span>
            <span class="truncate">{{ item.name }}</span>
            <span class="text-right text-fg-muted">
              {{ item.kind === "directory" ? "—" : formatBytes(item.size) }}
            </span>
            <span class="text-right text-fg-muted">
              {{ formatModifiedTime(item.modifiedUnixMs) }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
