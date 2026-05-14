<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useDebugStore } from "@/stores/debug";
import { useLiveProgressStore } from "@/stores/liveProgress";

const debug = useDebugStore();
const {
  lastProbe,
  lastProbeRoot,
  lastProbeError,
  lastTransfer,
  lastTransferSource,
  lastTransferDestination,
  lastTransferError,
} = storeToRefs(debug);

const liveProgress = useLiveProgressStore();
const { inFlight } = storeToRefs(liveProgress);

function flag(b: boolean): string {
  return b ? "✓" : "✗";
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1_048_576).toFixed(1)} MiB`;
  return `${(n / 1_073_741_824).toFixed(2)} GiB`;
}

function formatSpeed(bps: number): string {
  return `${formatBytes(bps)}/s`;
}

function percent(done: number, total: number): number {
  if (total <= 0) return 0;
  const pct = (done / total) * 100;
  if (pct < 0) return 0;
  if (pct > 100) return 100;
  return pct;
}

function formatEta(secs: number | null): string {
  if (secs === null || secs < 0) return "—";
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}m${s.toString().padStart(2, "0")}s`;
}

const capabilitiesLine = computed(() => {
  const caps = lastProbe.value;
  if (!caps) return "";
  return [
    `byte_range=${flag(caps.supportsByteRange)}`,
    `resume=${flag(caps.supportsResume)}`,
    `checksum=${flag(caps.supportsRemoteChecksum)}`,
    `server_rename=${flag(caps.supportsServerSideRename)}`,
    `symlinks=${flag(caps.supportsSymlinks)}`,
    `multipart=${flag(caps.supportsMultipart)}`,
    `max_sessions=${caps.maxParallelSessions}`,
  ].join(" ");
});

const transferLine = computed(() => {
  const report = lastTransfer.value;
  if (!report) return "";
  return [
    `id=${report.transferId.slice(0, 8)}`,
    `bytes=${formatBytes(report.bytesTransferred)}`,
    `time=${report.durationMs}ms`,
    `speed=${formatSpeed(report.avgSpeedBps)}`,
  ].join(" ");
});

const hasAnyState = computed(
  () =>
    !!lastProbe.value ||
    !!lastProbeError.value ||
    !!lastTransfer.value ||
    !!lastTransferError.value ||
    inFlight.value.length > 0,
);
</script>

<template>
  <div
    v-if="hasAnyState"
    class="flex shrink-0 flex-col gap-1 border-t border-border-muted bg-surface-sunken px-3 py-1 font-mono text-xs"
    role="status"
  >
    <div v-if="lastProbe || lastProbeError" class="flex items-center gap-3">
      <span class="shrink-0 text-fg-muted">probe:</span>
      <span class="truncate text-fg-subtle">{{ lastProbeRoot }}</span>
      <span class="text-fg-muted">·</span>
      <span v-if="lastProbeError" class="truncate text-status-danger">
        {{ lastProbeError }}
      </span>
      <span v-else class="truncate text-status-success">{{ capabilitiesLine }}</span>
      <button
        type="button"
        class="ml-auto shrink-0 rounded border border-border-muted px-2 py-0.5 text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
        @click="debug.clear()"
      >
        ×
      </button>
    </div>

    <div
      v-for="entry in inFlight"
      :key="entry.id"
      class="flex flex-col gap-0.5 border-t border-border-muted/50 pt-1"
    >
      <div class="flex items-center gap-3">
        <span class="shrink-0 text-fg-muted">transferring:</span>
        <span class="shrink-0 text-fg-subtle">{{ entry.id.slice(0, 8) }}</span>
        <span class="text-fg-muted">·</span>
        <span class="truncate text-fg-default">
          {{ formatBytes(entry.bytesDone) }} /
          {{ formatBytes(entry.bytesTotal) }}
          ({{ percent(entry.bytesDone, entry.bytesTotal).toFixed(1) }}%)
        </span>
        <span class="text-fg-muted">·</span>
        <span class="shrink-0 text-fg-subtle">
          {{ formatSpeed(entry.speedBps) }}
        </span>
        <span class="text-fg-muted">·</span>
        <span class="shrink-0 text-fg-subtle">
          ETA {{ formatEta(entry.etaSecs) }}
        </span>
      </div>
      <div class="h-1 w-full overflow-hidden rounded bg-surface-overlay">
        <div
          class="h-full bg-accent-default transition-[width] duration-150 ease-out"
          :style="{ width: `${percent(entry.bytesDone, entry.bytesTotal)}%` }"
        />
      </div>
    </div>

    <div
      v-if="lastTransfer || lastTransferError"
      class="flex items-center gap-3 border-t border-border-muted/50 pt-1"
    >
      <span class="shrink-0 text-fg-muted">transfer:</span>
      <span class="truncate text-fg-subtle">
        {{ lastTransferSource }} → {{ lastTransferDestination }}
      </span>
      <span class="text-fg-muted">·</span>
      <span v-if="lastTransferError" class="truncate text-status-danger">
        {{ lastTransferError }}
      </span>
      <span v-else class="truncate text-status-success">{{ transferLine }}</span>
    </div>
  </div>
</template>
