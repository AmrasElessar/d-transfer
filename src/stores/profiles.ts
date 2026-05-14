import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  AdapterCapabilities,
  ConnectionProfile,
} from "@/types";

function isTauriEnv(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * ConnectionProfile state + backend bridge (Bölüm 25).
 *
 * - `profiles`        : DB'den çekilmiş tüm profiller (name asc).
 * - `activeProfileId` : UI seçimi; persistans sonra. DualPane/QueuePanel buna bakar.
 * - `loadAll()`       : ilk açılışta / mutasyon sonrası refetch.
 * - `create/update`   : `secret` semantiği `None | Some("") | Some(value)`:
 *                        - undefined → secret alanına dokunma
 *                        - "" → silmek
 *                        - "value" → keystore'a yaz
 * - `testConnection`  : geçici adapter inşa eder, capability döner; persist yok.
 *
 * Tauri-dışı ortamda (browser dev) IPC çağrıları "not available" hatasıyla
 * reject olur; store boş kalır.
 */
export const useProfilesStore = defineStore("profiles", () => {
  const profiles = ref<ConnectionProfile[]>([]);
  const activeProfileId = ref<string | null>(null);
  const loading = ref(false);
  const lastError = ref<string | null>(null);

  const activeProfile = computed<ConnectionProfile | null>(
    () => profiles.value.find((p) => p.id === activeProfileId.value) ?? null,
  );

  function setActive(id: string | null) {
    activeProfileId.value = id;
  }

  async function loadAll(): Promise<void> {
    if (!isTauriEnv()) return;
    loading.value = true;
    try {
      const fetched = await invoke<ConnectionProfile[]>("list_profiles");
      profiles.value = fetched;
      // Aktif profil silinmiş olabilir — referansı koru veya temizle.
      if (
        activeProfileId.value !== null &&
        !fetched.some((p) => p.id === activeProfileId.value)
      ) {
        activeProfileId.value = null;
      }
      lastError.value = null;
    } catch (err) {
      lastError.value = formatError(err);
      console.warn("[profiles] list_profiles failed:", err);
    } finally {
      loading.value = false;
    }
  }

  async function create(
    profile: ConnectionProfile,
    secret: string | undefined,
  ): Promise<ConnectionProfile> {
    if (!isTauriEnv()) {
      throw new Error("Tauri IPC not available (non-Tauri runtime)");
    }
    const created = await invoke<ConnectionProfile>("create_profile", {
      profile,
      secret: secret ?? null,
    });
    await loadAll();
    activeProfileId.value = created.id;
    return created;
  }

  async function update(
    profile: ConnectionProfile,
    secret: string | undefined,
  ): Promise<ConnectionProfile> {
    if (!isTauriEnv()) {
      throw new Error("Tauri IPC not available (non-Tauri runtime)");
    }
    const updated = await invoke<ConnectionProfile>("update_profile", {
      profile,
      secret: secret ?? null,
    });
    await loadAll();
    return updated;
  }

  async function remove(id: string): Promise<void> {
    if (!isTauriEnv()) {
      throw new Error("Tauri IPC not available (non-Tauri runtime)");
    }
    await invoke<void>("delete_profile", { id });
    if (activeProfileId.value === id) {
      activeProfileId.value = null;
    }
    await loadAll();
  }

  async function testConnection(
    profile: ConnectionProfile,
    secret: string | undefined,
  ): Promise<AdapterCapabilities> {
    if (!isTauriEnv()) {
      throw new Error("Tauri IPC not available (non-Tauri runtime)");
    }
    return invoke<AdapterCapabilities>("test_connection", {
      profile,
      secret: secret ?? null,
    });
  }

  return {
    profiles,
    activeProfileId,
    activeProfile,
    loading,
    lastError,
    setActive,
    loadAll,
    create,
    update,
    remove,
    testConnection,
  };
});

function formatError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}
