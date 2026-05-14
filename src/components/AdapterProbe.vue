<script setup lang="ts">
import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useEngine } from "@/composables/useEngine";
import { useDebugStore } from "@/stores/debug";

const { probeLocalAdapter } = useEngine();
const debug = useDebugStore();
const busy = ref(false);

async function runProbe() {
  if (busy.value) return;
  busy.value = true;
  try {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected !== "string") {
      busy.value = false;
      return;
    }
    try {
      const caps = await probeLocalAdapter(selected);
      debug.setProbeResult(selected, caps);
    } catch (err) {
      const message =
        err && typeof err === "object" && "message" in err
          ? String((err as { message: unknown }).message)
          : String(err);
      debug.setProbeError(selected, message);
    }
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <button
    type="button"
    :disabled="busy"
    class="rounded-md border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default disabled:cursor-not-allowed disabled:opacity-50"
    @click="runProbe"
  >
    🔍 Test Local Adapter
  </button>
</template>
