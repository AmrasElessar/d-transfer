<script setup lang="ts">
import { onMounted, ref } from "vue";
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import { useProfilesStore } from "@/stores/profiles";
import type { ConnectionProfile, ProfileProtocol } from "@/types";
import ProfileDialog from "@/components/ProfileDialog.vue";

const { t } = useI18n();
const profilesStore = useProfilesStore();
const { profiles, activeProfileId, lastError } = storeToRefs(profilesStore);

const dialogOpen = ref(false);
const editingProfile = ref<ConnectionProfile | null>(null);

onMounted(() => {
  void profilesStore.loadAll();
});

function openNew() {
  editingProfile.value = null;
  dialogOpen.value = true;
}

function openEdit(profile: ConnectionProfile) {
  profilesStore.setActive(profile.id);
  editingProfile.value = profile;
  dialogOpen.value = true;
}

function closeDialog() {
  dialogOpen.value = false;
  editingProfile.value = null;
}

async function handleDelete(profile: ConnectionProfile, e: Event) {
  e.stopPropagation();
  // Native confirm — UI'da modal yerine OS dialog yeterli (settings panel
  // tarzında full-app dialog tek seferlik bu basit onay için overkill).
  if (!confirm(t("profiles.deleteConfirm"))) return;
  try {
    await profilesStore.remove(profile.id);
  } catch (err) {
    console.warn("[profiles] delete failed:", err);
  }
}

const PROTOCOL_BADGE_CLASS: Record<ProfileProtocol, string> = {
  local: "bg-status-success/15 text-status-success",
  sftp: "bg-accent-default/20 text-accent-default",
  s3: "bg-status-warning/15 text-status-warning",
  webdav: "bg-status-info/15 text-status-info",
};
</script>

<template>
  <aside
    class="flex min-h-0 flex-col rounded-md border border-border-muted bg-surface-raised"
    :aria-label="t('profiles.title')"
  >
    <header
      class="flex h-9 shrink-0 items-center justify-between border-b border-border-muted px-3 text-xs font-medium uppercase tracking-wider text-fg-muted"
    >
      <span>{{ t("profiles.title") }}</span>
      <button
        type="button"
        class="rounded border border-border-muted px-2 py-0.5 text-xs normal-case tracking-normal text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
        :aria-label="t('profiles.new')"
        @click="openNew"
      >
        +
      </button>
    </header>

    <div
      v-if="profiles.length === 0"
      class="flex flex-1 items-center justify-center px-3 py-4 text-center text-xs text-fg-subtle"
    >
      {{ t("profiles.noProfilesYet") }}
    </div>

    <ul v-else class="flex flex-1 flex-col divide-y divide-border-muted overflow-auto">
      <li
        v-for="profile in profiles"
        :key="profile.id"
        :class="[
          'group flex cursor-pointer items-center gap-2 px-3 py-2 text-xs transition-colors',
          activeProfileId === profile.id
            ? 'bg-surface-overlay text-fg-default'
            : 'text-fg-muted hover:bg-surface-overlay hover:text-fg-default',
        ]"
        :aria-current="activeProfileId === profile.id ? 'true' : undefined"
        @click="openEdit(profile)"
      >
        <span
          :class="[
            'inline-block rounded px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider',
            PROTOCOL_BADGE_CLASS[profile.protocol],
          ]"
        >
          {{ t(`profiles.protocols.${profile.protocol}`) }}
        </span>
        <span class="flex-1 truncate">{{ profile.name }}</span>
        <button
          type="button"
          class="invisible rounded px-1 text-fg-subtle hover:text-status-danger group-hover:visible"
          :aria-label="t('profiles.delete')"
          @click="(e) => handleDelete(profile, e)"
        >
          ×
        </button>
      </li>
    </ul>

    <div
      v-if="lastError"
      class="border-t border-status-danger/40 bg-status-danger/10 px-3 py-1 text-[11px] text-status-danger"
    >
      {{ lastError }}
    </div>

    <ProfileDialog
      :open="dialogOpen"
      :profile="editingProfile"
      @close="closeDialog"
    />
  </aside>
</template>
