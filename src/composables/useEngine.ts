import { onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  AdapterCapabilities,
  EngineStatus,
  LocalTransferReport,
  LocalTransferRequest,
  WireError,
} from "@/types";

const POLL_INTERVAL_MS = 1000;

function isTauriEnv(): boolean {
  // Tauri 2 injects __TAURI_INTERNALS__ into the window before app mount.
  // Tarayıcıdan `pnpm dev` ile açıldıysa bu yok — invoke atar.
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function formatError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as WireError).message);
  }
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}

export function useEngine() {
  const running = ref(false);
  const cancelled = ref(false);
  const subscriberCount = ref(0);
  const lastError = ref<string | null>(null);

  let pollHandle: ReturnType<typeof setTimeout> | null = null;
  let disposed = false;

  async function pollOnce(): Promise<void> {
    if (!isTauriEnv()) {
      running.value = false;
      lastError.value = "Tauri IPC not available (non-Tauri runtime)";
      return;
    }
    try {
      const status = await invoke<EngineStatus>("engine_status");
      running.value = status.running;
      cancelled.value = status.cancelled;
      subscriberCount.value = status.eventSubscribers;
      lastError.value = null;
    } catch (err) {
      running.value = false;
      lastError.value = formatError(err);
      console.warn("[useEngine] engine_status poll failed:", err);
    }
  }

  function scheduleNext(): void {
    if (disposed) return;
    pollHandle = setTimeout(async () => {
      await pollOnce();
      scheduleNext();
    }, POLL_INTERVAL_MS);
  }

  async function probeLocalAdapter(root: string): Promise<AdapterCapabilities> {
    if (!isTauriEnv()) {
      throw new Error("Tauri IPC not available (non-Tauri runtime)");
    }
    return invoke<AdapterCapabilities>("probe_local_adapter", { root });
  }

  async function runLocalTransfer(
    request: LocalTransferRequest,
  ): Promise<LocalTransferReport> {
    if (!isTauriEnv()) {
      throw new Error("Tauri IPC not available (non-Tauri runtime)");
    }
    return invoke<LocalTransferReport>("start_local_transfer", { request });
  }

  onMounted(async () => {
    await pollOnce();
    scheduleNext();
  });

  onUnmounted(() => {
    disposed = true;
    if (pollHandle !== null) {
      clearTimeout(pollHandle);
      pollHandle = null;
    }
  });

  return {
    running,
    cancelled,
    subscriberCount,
    lastError,
    probeLocalAdapter,
    runLocalTransfer,
  };
}
