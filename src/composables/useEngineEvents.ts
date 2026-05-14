import { onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useLiveProgressStore } from "@/stores/liveProgress";
import type { EngineEvent } from "@/types";

/**
 * Global singleton listener for the "engine-event" channel emitted by the Rust
 * bridge (lib.rs setup). App lifecycle boyunca yalnızca bir kez bağlanır;
 * birden çok bileşen `useEngineEvents()` çağırsa bile gerçek `listen()` tek
 * sefer kurulur.
 */
let mounted = false;
let unlisten: UnlistenFn | null = null;

function isTauriEnv(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function useEngineEvents() {
  const live = useLiveProgressStore();

  async function start(): Promise<void> {
    if (mounted) return;
    if (!isTauriEnv()) return;
    try {
      unlisten = await listen<EngineEvent>("engine-event", (event) => {
        live.apply(event.payload);
      });
      mounted = true;
    } catch (err) {
      console.warn("[useEngineEvents] failed to attach listener:", err);
    }
  }

  onMounted(start);
  onUnmounted(() => {
    if (unlisten) {
      unlisten();
      unlisten = null;
      mounted = false;
    }
  });
}
