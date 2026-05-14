<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { storeToRefs } from "pinia";
import { RecycleScroller } from "vue-virtual-scroller";
import "vue-virtual-scroller/dist/vue-virtual-scroller.css";
import { useRemoteBrowser, type SelectionMode } from "@/composables/useRemoteBrowser";
import { useProfilesStore } from "@/stores/profiles";
import { formatBytes, formatModifiedTime } from "@/utils/format";
import type { RemoteEntryDto } from "@/types";

const { t } = useI18n();

const profilesStore = useProfilesStore();
const { activeProfile } = storeToRefs(profilesStore);

const browser = useRemoteBrowser();

async function toggleHidden() {
  await browser.setIncludeHidden(!browser.includeHidden.value);
}

/**
 * POSIX-tabanlı breadcrumb. Remote path her zaman `/` separator'ı kullanır
 * (SFTP standardı; LocalAdapter relative path'leri root'a relatif). Kök item'i
 * `"/"` veya profile remoteRoot. Sıralı kümülatif segmentler tıklanabilir.
 */
const breadcrumb = computed<{ label: string; target: string }[]>(() => {
  const p = browser.cwd.value;
  if (!p) return [];

  // Absolute POSIX path → ["/", ...segments]
  if (p.startsWith("/")) {
    const parts = p.split("/").filter(Boolean);
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

  // Relative path (LocalAdapter root altında) — root rozet + segment'ler.
  if (p === "" || p === ".") {
    return [{ label: ".", target: "" }];
  }
  const parts = p.split("/").filter(Boolean);
  const items: { label: string; target: string }[] = [
    { label: ".", target: "" },
  ];
  let acc = "";
  for (const part of parts) {
    acc = acc.length === 0 ? part : `${acc}/${part}`;
    items.push({ label: part, target: acc });
  }
  return items;
});

async function navigateCrumb(target: string) {
  await browser.navigate(target);
}

// Click handler — LocalPane ile aynı pattern: manual double-click detection,
// modifier keys → selection mode.
let lastClickPath: string | null = null;
let lastClickAt = 0;
const DOUBLE_CLICK_MS = 350;

function onEntryClick(entry: RemoteEntryDto, event: MouseEvent) {
  const now = Date.now();
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

function entryGlyph(entry: RemoteEntryDto): string {
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

function isSelected(entry: RemoteEntryDto): boolean {
  return browser.selection.value.has(entry.path);
}

const VIRTUAL_THRESHOLD = 200;
const useVirtual = computed(() => browser.entries.value.length > VIRTUAL_THRESHOLD);
const ROW_HEIGHT = 24;
</script>

<template>
  <div
    class="flex min-h-0 flex-col rounded-sm border border-border-muted bg-surface-raised"
    :aria-label="t('panes.remote')"
  >
    <header
      class="flex shrink-0 flex-col gap-1 border-b border-border-muted px-2 py-1.5"
    >
      <div class="flex items-center gap-2">
        <!-- Profile badge solda (drive selector yerine) -->
        <span
          class="text-xs font-medium uppercase tracking-wider text-fg-muted"
        >
          {{ t("panes.remote") }}
        </span>
        <span
          v-if="activeProfile"
          class="rounded border border-border-muted bg-surface-overlay px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider text-fg-muted"
        >
          {{ activeProfile.protocol }}
        </span>
        <span
          v-if="activeProfile"
          class="truncate text-xs text-fg-default"
        >
          {{ activeProfile.name }}
        </span>

        <div class="flex-1" />

        <!-- Toolbar buttons — yalnızca bağlıyken interaktif -->
        <button
          type="button"
          class="rounded border border-transparent px-1.5 py-0.5 text-xs text-fg-muted hover:border-border-muted hover:bg-surface-overlay hover:text-fg-default"
          :title="t('remoteBrowser.actions.refresh')"
          :aria-label="t('remoteBrowser.actions.refresh')"
          :disabled="!browser.boundProfileId.value"
          :class="{
            'cursor-not-allowed opacity-40': !browser.boundProfileId.value,
          }"
          @click="browser.refresh()"
        >
          ↻
        </button>
        <button
          type="button"
          class="rounded border border-transparent px-1.5 py-0.5 text-xs text-fg-muted hover:border-border-muted hover:bg-surface-overlay hover:text-fg-default"
          :title="t('remoteBrowser.actions.up')"
          :aria-label="t('remoteBrowser.actions.up')"
          :disabled="!browser.parent.value"
          :class="{ 'cursor-not-allowed opacity-40': !browser.parent.value }"
          @click="browser.up()"
        >
          ↑
        </button>
        <button
          type="button"
          class="rounded border border-transparent px-1.5 py-0.5 text-xs hover:border-border-muted hover:bg-surface-overlay"
          :title="t('remoteBrowser.actions.toggleHidden')"
          :aria-label="t('remoteBrowser.actions.toggleHidden')"
          :aria-pressed="browser.includeHidden.value"
          :disabled="!browser.boundProfileId.value"
          :class="[
            browser.includeHidden.value
              ? 'text-accent-default'
              : 'text-fg-muted hover:text-fg-default',
            !browser.boundProfileId.value ? 'cursor-not-allowed opacity-40' : '',
          ]"
          @click="toggleHidden()"
        >
          ⊙
        </button>
      </div>

      <!-- Breadcrumb (yalnızca bağlıyken) -->
      <nav
        v-if="browser.boundProfileId.value"
        class="flex items-center gap-0.5 overflow-x-auto whitespace-nowrap font-mono text-xs text-fg-muted"
        :aria-label="t('panes.remote')"
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
        <span
          v-if="breadcrumb.length === 0 && browser.loading.value"
          class="px-1"
        >
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
      {{ t("remoteBrowser.errorPrefix") }}: {{ browser.error.value }}
    </div>

    <!-- No profile selected -->
    <div
      v-if="!activeProfile"
      class="flex flex-1 items-center justify-center text-xs text-fg-subtle"
    >
      {{ t("remoteBrowser.noProfileSelected") }}
    </div>

    <!-- Connecting (initial) -->
    <div
      v-else-if="browser.connecting.value && !browser.cwd.value"
      class="flex flex-1 items-center justify-center text-xs text-fg-subtle"
    >
      {{ t("remoteBrowser.connecting") }}
    </div>

    <!-- Body -->
    <div v-else class="flex min-h-0 flex-1 flex-col">
      <!-- Column headers -->
      <div
        class="grid shrink-0 grid-cols-[1.5rem_1fr_6rem_8rem] gap-2 border-b border-border-muted px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-fg-subtle"
      >
        <span />
        <span>{{ t("localBrowser.columns.name") }}</span>
        <span class="text-right">{{ t("localBrowser.columns.size") }}</span>
        <span class="text-right">{{
          t("localBrowser.columns.modified")
        }}</span>
      </div>

      <!-- Empty dir -->
      <div
        v-if="
          !browser.loading.value &&
          !browser.error.value &&
          browser.entries.value.length === 0 &&
          !browser.parent.value
        "
        class="flex flex-1 items-center justify-center text-xs text-fg-subtle"
      >
        {{ t("remoteBrowser.emptyDir") }}
      </div>

      <div v-else class="min-h-0 flex-1 overflow-hidden font-mono text-xs">
        <!-- Parent ".." -->
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
              isSelected(item)
                ? 'bg-accent-subtle text-fg-default'
                : 'hover:bg-surface-overlay',
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

        <div v-else class="h-full overflow-auto">
          <div
            v-for="item in browser.entries.value"
            :key="item.path"
            class="grid cursor-pointer grid-cols-[1.5rem_1fr_6rem_8rem] items-center gap-2 px-2 py-1"
            :style="{ height: ROW_HEIGHT + 'px' }"
            :class="[
              isSelected(item)
                ? 'bg-accent-subtle text-fg-default'
                : 'hover:bg-surface-overlay',
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
