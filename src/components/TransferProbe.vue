<script setup lang="ts">
import { computed, ref } from "vue";
import { storeToRefs } from "pinia";
import { open } from "@tauri-apps/plugin-dialog";
import { useEngine } from "@/composables/useEngine";
import { useDebugStore } from "@/stores/debug";

const { runLocalTransfer } = useEngine();
const debug = useDebugStore();
const { lastProbe, lastProbeRoot } = storeToRefs(debug);

const busy = ref(false);

// Probe önce çalıştırılıp root bilinmelidir; aksi halde buton disable.
const canRun = computed(() => !!lastProbe.value && !!lastProbeRoot.value);

function basename(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const lastSep = trimmed.search(/[\\/](?!.*[\\/])/);
  return lastSep >= 0 ? trimmed.slice(lastSep + 1) : trimmed;
}

function timestampSuffix(): string {
  return new Date().toISOString().replace(/[:.]/g, "-");
}

async function runTransfer() {
  if (!canRun.value || busy.value || !lastProbeRoot.value) return;
  busy.value = true;
  try {
    const source = await open({ directory: false, multiple: false });
    if (typeof source !== "string") {
      busy.value = false;
      return;
    }
    const destination = `${basename(source)}.${timestampSuffix()}.copy`;
    try {
      const report = await runLocalTransfer({
        root: lastProbeRoot.value,
        source,
        destination,
      });
      debug.setTransferResult(source, destination, report);
    } catch (err) {
      const message =
        err && typeof err === "object" && "message" in err
          ? String((err as { message: unknown }).message)
          : String(err);
      debug.setTransferError(source, destination, message);
    }
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <button
    type="button"
    :disabled="!canRun || busy"
    :title="canRun ? '' : 'Run probe first'"
    class="rounded-md border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default disabled:cursor-not-allowed disabled:opacity-50"
    @click="runTransfer"
  >
    🚚 Run Test Transfer
  </button>
</template>
