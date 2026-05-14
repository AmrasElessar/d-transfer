import { defineStore } from "pinia";
import { computed, ref } from "vue";

/** Faz 1 placeholder. Asıl model Faz 2'de Rust tarafıyla şekillenir. */
export type Protocol = "sftp" | "s3" | "webdav" | "local";

export interface ConnectionProfile {
  id: string;
  name: string;
  protocol: Protocol;
  host: string;
  username: string | null;
}

export const useConnectionStore = defineStore("connection", () => {
  const profiles = ref<ConnectionProfile[]>([]);
  const activeProfileId = ref<string | null>(null);

  const activeProfile = computed(() =>
    profiles.value.find((p) => p.id === activeProfileId.value) ?? null,
  );

  function setActive(id: string | null) {
    activeProfileId.value = id;
  }

  return { profiles, activeProfileId, activeProfile, setActive };
});
