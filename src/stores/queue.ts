import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  EngineEvent,
  TransferDirection,
  TransferDto,
  TransferState,
  WireError,
} from "@/types";

/**
 * UI queue panel'in canonical kaynağı.
 *
 * İki giriş yolu:
 * - `loadAll()` — mount'ta IPC ile DB'den tam liste; backend `list_transfers`
 *   en son 200 satırı state-priority + created_at DESC sırasıyla döner.
 * - `applyEvent()` — `useEngineEvents` runtime'da `transferStateChanged`,
 *   `transferProgress`, `transferCompleted`, `transferFailed` event'lerini
 *   buraya forward eder; mevcut satır upsert edilir, yoksa partial bilgiyle
 *   geçici satır oluşur (sonraki loadAll'da tamamlanır).
 */
export interface QueueItem {
  id: string;
  profileId: string | null;
  direction: TransferDirection;
  source: string;
  target: string;
  state: TransferState;
  bytesTotal: number;
  bytesDone: number;
  /** Engine progress event'lerinden gelir; DB'de saklanmaz. */
  speedBps: number;
  /** Engine progress event'lerinden gelir; null = bilinmiyor. */
  etaSecs: number | null;
  /** Sadece terminal `failed` durumunda dolu. */
  error: WireError | null;
  createdAt: number;
  /** Active geçişinde dolar. */
  startedAt: number | null;
  /** Terminal state (completed/failed/cancelled/skipped) anında dolar. */
  finishedAt: number | null;
}

function isTauriEnv(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function dtoToItem(dto: TransferDto): QueueItem {
  return {
    id: dto.id,
    profileId: dto.profileId,
    direction: dto.direction,
    source: dto.localPath,
    target: dto.remotePath,
    state: dto.state,
    bytesTotal: dto.bytesTotal,
    bytesDone: dto.bytesDone,
    speedBps: 0,
    etaSecs: null,
    error: dto.lastError ? parseWireErrorJson(dto.lastError) : null,
    createdAt: Date.parse(dto.createdAt),
    startedAt: dto.startedAt ? Date.parse(dto.startedAt) : null,
    finishedAt: dto.completedAt ? Date.parse(dto.completedAt) : null,
  };
}

/** Rust son hata stringini JSON `WireError` formunda saklar; UI plain ise wrap'ler. */
function parseWireErrorJson(raw: string): WireError {
  try {
    return JSON.parse(raw) as WireError;
  } catch {
    return {
      category: "unknown",
      suggestedAction: "userDecision",
      i18nKey: "unknown",
      message: raw,
    };
  }
}

function isTerminalState(state: TransferState): boolean {
  return (
    state === "completed" ||
    state === "failed" ||
    state === "cancelled" ||
    state === "skipped"
  );
}

function ensureItem(
  items: Map<string, QueueItem>,
  id: string,
  defaults: Partial<QueueItem>,
): QueueItem {
  const existing = items.get(id);
  if (existing) return existing;
  const fresh: QueueItem = {
    id,
    profileId: null,
    direction: "upload",
    source: "",
    target: "",
    state: "queued",
    bytesTotal: 0,
    bytesDone: 0,
    speedBps: 0,
    etaSecs: null,
    error: null,
    createdAt: Date.now(),
    startedAt: null,
    finishedAt: null,
    ...defaults,
  };
  items.set(id, fresh);
  return fresh;
}

export const useQueueStore = defineStore("queue", () => {
  const itemsMap = ref<Map<string, QueueItem>>(new Map());
  /** Vue 3 Map mutasyonlarını derinden izlemediği için counter ile invalidate. */
  const version = ref(0);
  const lastError = ref<string | null>(null);

  function bump(): void {
    version.value += 1;
  }

  const items = computed<QueueItem[]>(() => {
    void version.value;
    const list = Array.from(itemsMap.value.values());
    // state priority: active grubu önce, queued ortada, paused sonra, terminal en altta.
    const order: Record<TransferState, number> = {
      active: 0,
      verifying: 0,
      finalizing: 0,
      queued: 1,
      paused: 2,
      completed: 3,
      failed: 3,
      cancelled: 3,
      skipped: 3,
    };
    list.sort((a, b) => {
      const da = order[a.state] - order[b.state];
      if (da !== 0) return da;
      return b.createdAt - a.createdAt;
    });
    return list;
  });

  const activeCount = computed(() => {
    void version.value;
    let n = 0;
    for (const it of itemsMap.value.values()) {
      if (it.state === "active" || it.state === "verifying" || it.state === "finalizing") {
        n += 1;
      }
    }
    return n;
  });

  const queuedCount = computed(() => {
    void version.value;
    let n = 0;
    for (const it of itemsMap.value.values()) {
      if (it.state === "queued") n += 1;
    }
    return n;
  });

  async function loadAll(limit = 200): Promise<void> {
    if (!isTauriEnv()) return;
    try {
      const rows = await invoke<TransferDto[]>("list_transfers", { limit });
      const next = new Map<string, QueueItem>();
      for (const dto of rows) next.set(dto.id, dtoToItem(dto));
      itemsMap.value = next;
      lastError.value = null;
      bump();
    } catch (err) {
      lastError.value = String(err);
      console.warn("[queue] list_transfers failed:", err);
    }
  }

  function applyEvent(event: EngineEvent): void {
    switch (event.type) {
      case "transferStateChanged": {
        const item = ensureItem(itemsMap.value, event.transferId, {});
        item.state = event.newState;
        if (event.newState === "active" && item.startedAt === null) {
          item.startedAt = Date.now();
        }
        if (isTerminalState(event.newState) && item.finishedAt === null) {
          item.finishedAt = Date.now();
        }
        bump();
        break;
      }
      case "transferProgress": {
        const item = ensureItem(itemsMap.value, event.transferId, { state: "active" });
        item.bytesDone = event.bytesDone;
        item.bytesTotal = event.bytesTotal;
        item.speedBps = event.speedBps;
        item.etaSecs = event.etaSecs;
        bump();
        break;
      }
      case "transferCompleted": {
        const item = ensureItem(itemsMap.value, event.transferId, { state: "completed" });
        item.state = "completed";
        item.finishedAt = Date.now();
        if (item.bytesTotal > 0) item.bytesDone = item.bytesTotal;
        item.speedBps = 0;
        item.etaSecs = null;
        bump();
        break;
      }
      case "transferFailed": {
        const item = ensureItem(itemsMap.value, event.transferId, { state: "failed" });
        item.state = "failed";
        item.error = event.error;
        item.finishedAt = Date.now();
        item.speedBps = 0;
        item.etaSecs = null;
        bump();
        break;
      }
      // Diğer event'ler (rateLimited, connection*, queue*, diagnostics) bu
      // store'da görüntülenmez; ileride ayrı bir notification stream'ine bağlanır.
      default:
        break;
    }
  }

  function remove(id: string): void {
    if (itemsMap.value.delete(id)) bump();
  }

  function clearTerminal(): void {
    let changed = false;
    for (const [id, item] of itemsMap.value) {
      if (isTerminalState(item.state)) {
        itemsMap.value.delete(id);
        changed = true;
      }
    }
    if (changed) bump();
  }

  return {
    items,
    activeCount,
    queuedCount,
    lastError,
    loadAll,
    applyEvent,
    remove,
    clearTerminal,
  };
});
