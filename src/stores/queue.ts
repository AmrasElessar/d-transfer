import { defineStore } from "pinia";
import { computed, ref } from "vue";

/** Faz 1 placeholder. Rust tarafındaki TransferState'in JSON projeksiyonu (Bölüm 15.2). */
export type TransferState =
  | "queued"
  | "active"
  | "verifying"
  | "finalizing"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled"
  | "skipped";

export interface QueueItem {
  id: string;
  direction: "upload" | "download";
  source: string;
  target: string;
  bytesTotal: number;
  bytesDone: number;
  state: TransferState;
}

export const useQueueStore = defineStore("queue", () => {
  const items = ref<QueueItem[]>([]);

  const activeCount = computed(
    () => items.value.filter((i) => i.state === "active").length,
  );

  const queuedCount = computed(
    () => items.value.filter((i) => i.state === "queued").length,
  );

  function upsert(item: QueueItem) {
    const idx = items.value.findIndex((i) => i.id === item.id);
    if (idx >= 0) items.value[idx] = item;
    else items.value.push(item);
  }

  function remove(id: string) {
    items.value = items.value.filter((i) => i.id !== id);
  }

  function clear() {
    items.value = [];
  }

  return { items, activeCount, queuedCount, upsert, remove, clear };
});
