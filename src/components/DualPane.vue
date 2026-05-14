<script setup lang="ts">
import LocalPane from "@/components/LocalPane.vue";
import RemotePane from "@/components/RemotePane.vue";
</script>

<template>
  <section
    class="grid min-h-0 grid-cols-2 gap-1 rounded-md border border-border-muted bg-surface-sunken p-1"
  >
    <LocalPane />
    <RemotePane />
  </section>
</template>
