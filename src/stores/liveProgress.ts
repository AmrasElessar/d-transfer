import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { EngineEvent, TransferState, WireError } from "@/types";

/**
 * Per-transferId in-flight progress state.
 *
 * Engine event'leri stream'inden türetilir; UI bunu live progress bar / row
 * göstermek için tüketir. Tamamlanan/başarısız transfer'ler `finishedAt`
 * dolduğunda hâlâ tutulur — caller `clear(id)` ile temizleyebilir.
 */
export interface LiveTransferState {
  state: TransferState;
  bytesDone: number;
  bytesTotal: number;
  speedBps: number;
  etaSecs: number | null;
  startedAt: number;
  finishedAt: number | null;
  error: WireError | null;
}

function initialEntry(state: TransferState): LiveTransferState {
  return {
    state,
    bytesDone: 0,
    bytesTotal: 0,
    speedBps: 0,
    etaSecs: null,
    startedAt: Date.now(),
    finishedAt: null,
    error: null,
  };
}

export const useLiveProgressStore = defineStore("liveProgress", () => {
  const transfers = ref<Map<string, LiveTransferState>>(new Map());
  // Vue 3 reactivity Map mutation'larını derinden izlemez; her apply() sonrası
  // bu counter artırılır → computed'lar invalidate olur.
  const version = ref(0);

  function bump(): void {
    version.value += 1;
  }

  function apply(event: EngineEvent): void {
    switch (event.type) {
      case "transferStateChanged": {
        const existing = transfers.value.get(event.transferId);
        if (existing) {
          existing.state = event.newState;
          if (
            event.newState === "completed" ||
            event.newState === "failed" ||
            event.newState === "cancelled"
          ) {
            existing.finishedAt = Date.now();
          }
        } else {
          transfers.value.set(event.transferId, initialEntry(event.newState));
        }
        bump();
        break;
      }
      case "transferProgress": {
        const entry =
          transfers.value.get(event.transferId) ?? initialEntry("active");
        entry.bytesDone = event.bytesDone;
        entry.bytesTotal = event.bytesTotal;
        entry.speedBps = event.speedBps;
        entry.etaSecs = event.etaSecs;
        transfers.value.set(event.transferId, entry);
        bump();
        break;
      }
      case "transferCompleted": {
        const entry =
          transfers.value.get(event.transferId) ?? initialEntry("completed");
        entry.state = "completed";
        entry.finishedAt = Date.now();
        // Completed implies bytesDone == bytesTotal; engine son progress tick'i
        // attığı için tipik olarak zaten doğru, ama defansif olarak normalize.
        if (entry.bytesTotal > 0) {
          entry.bytesDone = entry.bytesTotal;
        }
        transfers.value.set(event.transferId, entry);
        bump();
        break;
      }
      case "transferFailed": {
        const entry =
          transfers.value.get(event.transferId) ?? initialEntry("failed");
        entry.state = "failed";
        entry.error = event.error;
        entry.finishedAt = Date.now();
        transfers.value.set(event.transferId, entry);
        bump();
        break;
      }
      // Diğer event türleri (rateLimited / connection* / queue* / shutdown /
      // diagnostics) Faz 3+'ta ayrı store'larda işlenecek.
      default:
        break;
    }
  }

  function get(id: string): LiveTransferState | undefined {
    // version okunarak reactive dependency kurulur.
    void version.value;
    return transfers.value.get(id);
  }

  function clear(id?: string): void {
    if (id === undefined) {
      transfers.value.clear();
    } else {
      transfers.value.delete(id);
    }
    bump();
  }

  const inFlight = computed<Array<{ id: string } & LiveTransferState>>(() => {
    void version.value;
    const out: Array<{ id: string } & LiveTransferState> = [];
    for (const [id, entry] of transfers.value) {
      if (
        entry.state === "active" ||
        entry.state === "verifying" ||
        entry.state === "finalizing"
      ) {
        out.push({ id, ...entry });
      }
    }
    return out;
  });

  return { transfers, version, inFlight, apply, get, clear };
});
