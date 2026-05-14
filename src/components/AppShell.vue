<script setup lang="ts">
import TitleBar from "@/components/TitleBar.vue";
import DualPane from "@/components/DualPane.vue";
import QueuePanel from "@/components/QueuePanel.vue";
import DebugBar from "@/components/DebugBar.vue";
import SettingsPanel from "@/components/SettingsPanel.vue";
import ProfileSidebar from "@/components/ProfileSidebar.vue";
</script>

<template>
  <div class="flex h-full w-full flex-col bg-surface-base text-fg-default">
    <TitleBar />
    <!-- 240px sabit sol kolon = profil sidebar; sağ tarafta DualPane + Queue
         dikey iki-row grid içinde. Sidebar her iki row'u kapsar (row-span-2). -->
    <main class="grid min-h-0 flex-1 grid-cols-[240px_1fr] grid-rows-[1fr_minmax(180px,30%)] gap-1 p-1">
      <ProfileSidebar class="row-span-2" />
      <DualPane />
      <QueuePanel />
    </main>
    <DebugBar />
    <!-- SettingsPanel kendi görünürlüğünü useSettingsPanel composable'ı üzerinden
         yönetir; prop/emit zinciri yok. -->
    <SettingsPanel />
  </div>
</template>
