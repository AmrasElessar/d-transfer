import { defineStore } from "pinia";
import { ref } from "vue";
import type { AdapterCapabilities, LocalTransferReport } from "@/types";

/**
 * Ephemeral debug store — Faz 2 boyunca elle test akışları için.
 * Disk'e persist edilmez; sadece in-memory.
 */
export const useDebugStore = defineStore("debug", () => {
  const lastProbe = ref<AdapterCapabilities | null>(null);
  const lastProbeRoot = ref<string | null>(null);
  const lastProbeError = ref<string | null>(null);

  const lastTransfer = ref<LocalTransferReport | null>(null);
  const lastTransferSource = ref<string | null>(null);
  const lastTransferDestination = ref<string | null>(null);
  const lastTransferError = ref<string | null>(null);

  function setProbeResult(root: string, caps: AdapterCapabilities) {
    lastProbeRoot.value = root;
    lastProbe.value = caps;
    lastProbeError.value = null;
  }

  function setProbeError(root: string, message: string) {
    lastProbeRoot.value = root;
    lastProbe.value = null;
    lastProbeError.value = message;
  }

  function setTransferResult(
    source: string,
    destination: string,
    report: LocalTransferReport,
  ) {
    lastTransferSource.value = source;
    lastTransferDestination.value = destination;
    lastTransfer.value = report;
    lastTransferError.value = null;
  }

  function setTransferError(
    source: string,
    destination: string,
    message: string,
  ) {
    lastTransferSource.value = source;
    lastTransferDestination.value = destination;
    lastTransfer.value = null;
    lastTransferError.value = message;
  }

  function clear() {
    lastProbe.value = null;
    lastProbeRoot.value = null;
    lastProbeError.value = null;
    lastTransfer.value = null;
    lastTransferSource.value = null;
    lastTransferDestination.value = null;
    lastTransferError.value = null;
  }

  return {
    lastProbe,
    lastProbeRoot,
    lastProbeError,
    lastTransfer,
    lastTransferSource,
    lastTransferDestination,
    lastTransferError,
    setProbeResult,
    setProbeError,
    setTransferResult,
    setTransferError,
    clear,
  };
});
