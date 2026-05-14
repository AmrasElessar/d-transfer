<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { useQueueStore } from "@/stores/queue";

const { t } = useI18n();
const queue = useQueueStore();
const { items, activeCount, queuedCount } = storeToRefs(queue);
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
      </span>
    </header>

    <div v-if="items.length === 0" class="flex flex-1 items-center justify-center text-xs text-fg-subtle">
      {{ t("queue.empty") }}
    </div>
    <ul v-else class="flex flex-1 flex-col divide-y divide-border-muted overflow-auto">
      <li
        v-for="item in items"
        :key="item.id"
        class="flex items-center gap-3 px-3 py-2 text-xs"
      >
        <span class="font-mono text-fg-muted">{{ item.direction }}</span>
        <span class="flex-1 truncate">{{ item.source }} → {{ item.target }}</span>
        <span class="text-fg-muted">{{ t(`queue.state.${item.state}`) }}</span>
      </li>
    </ul>
  </section>
</template>
