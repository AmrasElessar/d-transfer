<script setup lang="ts">
import { onMounted, ref } from "vue";
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useQueueStore } from "@/stores/queue";
import {
  formatBytes,
  formatEta,
  formatSpeed,
  progressPercent,
} from "@/utils/format";
import type { TransferState } from "@/types";

const { t } = useI18n();
const queue = useQueueStore();
const { items, activeCount, queuedCount, lastError } = storeToRefs(queue);

const cancelling = ref<Set<string>>(new Set());

onMounted(() => {
  void queue.loadAll();
});

function canCancel(state: TransferState): boolean {
  return state === "active" || state === "verifying" || state === "finalizing" || state === "queued";
}

async function cancelTransfer(id: string): Promise<void> {
  if (cancelling.value.has(id)) return;
  cancelling.value.add(id);
  // Set mutation reactivity için yeni Set ata
  cancelling.value = new Set(cancelling.value);
  try {
    await invoke<{ cancelled: boolean }>("cancel_transfer", { transferId: id });
    // Engine event'i state'i Cancelled'a çekecek; UI hemen `cancelling`
    // göstergesini bırakıyor. Tutmaya değmez — race koşulları minimal.
  } catch (err) {
    console.warn("[queue] cancel failed:", err);
  } finally {
    cancelling.value.delete(id);
    cancelling.value = new Set(cancelling.value);
  }
}

// Çift slash'tan sonraki son segment — uzun path'lerde okunur kalmak için.
function basename(path: string): string {
  if (!path) return "";
  const norm = path.replace(/\\/g, "/").replace(/\/+$/, "");
  const idx = norm.lastIndexOf("/");
  return idx >= 0 ? norm.slice(idx + 1) : norm;
}

function stateClass(s: TransferState): string {
  switch (s) {
    case "active":
    case "verifying":
    case "finalizing":
      return "text-accent-default";
    case "completed":
      return "text-status-success";
    case "failed":
    case "cancelled":
      return "text-status-danger";
    case "paused":
      return "text-status-warning";
    case "skipped":
      return "text-fg-subtle";
    default:
      return "text-fg-muted";
  }
}

function progressBarClass(s: TransferState): string {
  switch (s) {
    case "completed":
      return "bg-status-success";
    case "failed":
    case "cancelled":
      return "bg-status-danger";
    case "paused":
      return "bg-status-warning";
    default:
      return "bg-accent-default";
  }
}
</script>

<template>
  <section
    class="flex min-h-0 flex-col rounded-md border border-border-muted bg-surface-raised"
    :aria-label="t('queue.title')"
  >
    <header
      class="flex h-9 shrink-0 items-center justify-between border-b border-border-muted px-3 text-xs font-medium uppercase tracking-wider text-fg-muted"
    >
      <span>{{ t("queue.title") }}</span>
      <span class="flex items-center gap-3 normal-case tracking-normal">
        <span>{{ t("queue.activeCount", { count: activeCount }) }}</span>
        <span class="text-fg-subtle">·</span>
        <span>{{ t("queue.queuedCount", { count: queuedCount }) }}</span>
        <button
          v-if="items.length > 0"
          type="button"
          class="ml-2 rounded border border-border-muted px-2 py-0.5 text-[10px] normal-case tracking-normal text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
          :aria-label="t('queue.clearTerminal')"
          @click="queue.clearTerminal()"
        >
          {{ t("queue.clearTerminal") }}
        </button>
      </span>
    </header>

    <div
      v-if="items.length === 0"
      class="flex flex-1 items-center justify-center text-xs text-fg-subtle"
    >
      {{ t("queue.empty") }}
    </div>
    <ul v-else class="flex flex-1 flex-col divide-y divide-border-muted overflow-auto">
      <li
        v-for="item in items"
        :key="item.id"
        class="flex flex-col gap-1 px-3 py-2 text-xs"
        role="row"
        :aria-label="`${item.direction} ${basename(item.source)} ${item.state}`"
      >
        <div class="flex items-center gap-3">
          <span
            class="font-mono text-[10px] uppercase tracking-wider"
            :class="item.direction === 'upload' ? 'text-accent-default' : 'text-status-info'"
          >
            {{ item.direction === "upload" ? "↑" : "↓" }}
          </span>
          <span class="flex-1 truncate font-mono">{{ basename(item.source) || basename(item.target) || item.id.slice(0, 8) }}</span>
          <span class="text-fg-subtle">
            {{ formatBytes(item.bytesDone) }}
            <template v-if="item.bytesTotal > 0">/ {{ formatBytes(item.bytesTotal) }}</template>
          </span>
          <span class="w-20 text-right text-fg-muted">{{ formatSpeed(item.speedBps) }}</span>
          <span class="w-14 text-right text-fg-muted">{{ formatEta(item.etaSecs) }}</span>
          <span
            class="w-20 text-right font-medium"
            :class="stateClass(item.state)"
          >
            {{ t(`queue.state.${item.state}`) }}
          </span>
          <button
            v-if="canCancel(item.state)"
            type="button"
            class="rounded border border-border-muted px-1.5 text-[10px] text-fg-muted hover:border-status-danger hover:text-status-danger disabled:cursor-not-allowed disabled:opacity-40"
            :title="t('queue.cancel')"
            :aria-label="t('queue.cancel')"
            :disabled="cancelling.has(item.id)"
            @click="cancelTransfer(item.id)"
          >
            ×
          </button>
          <button
            v-else-if="item.state === 'completed' || item.state === 'failed' || item.state === 'cancelled' || item.state === 'skipped'"
            type="button"
            class="rounded border border-transparent px-1.5 text-[10px] text-fg-subtle hover:border-border-muted hover:text-fg-muted"
            :title="t('queue.clearTerminal')"
            :aria-label="t('queue.clearTerminal')"
            @click="queue.remove(item.id)"
          >
            ✕
          </button>
          <span v-else class="inline-block w-6" />
        </div>

        <!-- Progress bar — bytesTotal > 0 ise yüzde göster; aksi halde indeterminate.
             Terminal state'lerde tam dolu / kırmızı çubuk göster. -->
        <div
          v-if="item.bytesTotal > 0 || item.state === 'completed' || item.state === 'failed' || item.state === 'cancelled'"
          class="h-1 w-full overflow-hidden rounded bg-surface-base"
          role="progressbar"
          :aria-valuenow="progressPercent(item.bytesDone, item.bytesTotal) ?? 0"
          aria-valuemin="0"
          aria-valuemax="100"
        >
          <div
            class="h-full transition-all"
            :class="progressBarClass(item.state)"
            :style="{
              width:
                item.state === 'completed'
                  ? '100%'
                  : item.state === 'failed' || item.state === 'cancelled'
                  ? `${progressPercent(item.bytesDone, item.bytesTotal) ?? 0}%`
                  : `${progressPercent(item.bytesDone, item.bytesTotal) ?? 0}%`,
            }"
          />
        </div>

        <div v-if="item.error" class="text-status-danger">
          {{ item.error.message }}
        </div>
      </li>
    </ul>

    <div
      v-if="lastError"
      class="border-t border-status-danger/40 bg-status-danger/10 px-3 py-1 text-[11px] text-status-danger"
    >
      {{ lastError }}
    </div>
  </section>
</template>
