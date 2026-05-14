import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { storeToRefs } from "pinia";
import { useProfilesStore } from "@/stores/profiles";
import type {
  AdapterCapabilities,
  ConnectionProfile,
  ListRemoteDirRequest,
  ListRemoteDirResponse,
  RemoteEntryDto,
  WireError,
} from "@/types";

/**
 * Remote dosya sistemi browser state'i + `ConnectionManager` IPC sarmalı.
 *
 * Davranış (Faz 4):
 * - Active profile değiştiğinde otomatik connect + initial listing.
 * - Connect **sticky**: aynı profile için tekrar tekrar IPC çağrılsa bile backend
 *   cache döner; ilk SSH handshake'i hariç pahalı bir çağrı yok.
 * - Profile None'a düşerse eski bağlantı `disconnect_profile` ile temizlenir.
 *
 * Selection / hidden-toggle pattern'ini `useLocalBrowser`'dan ödünç alıyoruz;
 * davranış parity = UI tutarlılığı.
 */
function isTauriEnv(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function formatError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as WireError).message);
  }
  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}

export type SelectionMode = "single" | "ctrl" | "shift";

/**
 * Profil için başlangıç path'ini hesapla — Local için `localRoot`, uzak protokoller
 * için `remoteRoot`. Hiçbiri yoksa `/` (uzak için POSIX root) veya `""` (Local
 * için adapter relative root). SFTP adapter relative `"."` veya `""` kabul eder.
 */
function initialPathFor(profile: ConnectionProfile): string {
  if (profile.protocol === "local") {
    // LocalAdapter root'una bağlanıldı; relative başlangıç noktası boş = root.
    return "";
  }
  return profile.remoteRoot ?? "/";
}

export function useRemoteBrowser() {
  const profilesStore = useProfilesStore();
  const { activeProfileId, activeProfile } = storeToRefs(profilesStore);

  // Hangi profil'e bağlı olduğumuzu izle — switch'te disconnect tetikleriz.
  const boundProfileId = ref<string | null>(null);
  const cwd = ref<string>("");
  const entries = ref<RemoteEntryDto[]>([]);
  const parent = ref<string | null>(null);
  const loading = ref(false);
  const connecting = ref(false);
  const error = ref<string | null>(null);
  const includeHidden = ref(false);
  const selection = ref<Set<string>>(new Set());
  const capabilities = ref<AdapterCapabilities | null>(null);

  let anchorPath: string | null = null;

  function resetState() {
    cwd.value = "";
    entries.value = [];
    parent.value = null;
    error.value = null;
    selection.value = new Set();
    anchorPath = null;
    capabilities.value = null;
  }

  async function fetchDir(profileId: string, target: string): Promise<void> {
    if (!isTauriEnv()) {
      error.value = "Tauri runtime gerekli";
      entries.value = [];
      parent.value = null;
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      const request: ListRemoteDirRequest = {
        profileId,
        path: target,
        includeHidden: includeHidden.value,
      };
      const resp = await invoke<ListRemoteDirResponse>("list_remote_dir", {
        request,
      });
      cwd.value = resp.path;
      entries.value = resp.entries;
      parent.value = resp.parent;
      selection.value = new Set();
      anchorPath = null;
    } catch (err) {
      error.value = formatError(err);
      entries.value = [];
    } finally {
      loading.value = false;
    }
  }

  async function connect(profileId: string): Promise<void> {
    if (!isTauriEnv()) {
      error.value = "Tauri runtime gerekli";
      return;
    }
    connecting.value = true;
    error.value = null;
    try {
      capabilities.value = await invoke<AdapterCapabilities>("connect_profile", {
        profileId,
      });
    } catch (err) {
      error.value = formatError(err);
      capabilities.value = null;
      throw err;
    } finally {
      connecting.value = false;
    }
  }

  async function disconnect(): Promise<void> {
    const id = boundProfileId.value;
    if (!id || !isTauriEnv()) {
      boundProfileId.value = null;
      resetState();
      return;
    }
    try {
      await invoke<void>("disconnect_profile", { profileId: id });
    } catch (err) {
      // Disconnect best-effort; backend cache zaten temiz olabilir.
      console.warn("[remote-browser] disconnect failed:", err);
    } finally {
      boundProfileId.value = null;
      resetState();
    }
  }

  async function bindToProfile(profileId: string, initialPath: string): Promise<void> {
    // Aynı profile'a yeniden bind = no-op (connect zaten cached).
    if (boundProfileId.value === profileId) {
      if (cwd.value === "") {
        await fetchDir(profileId, initialPath);
      }
      return;
    }
    // Farklı bir profile'a geçiyorsak eskisini bırak.
    if (boundProfileId.value && boundProfileId.value !== profileId) {
      await disconnect();
    }
    boundProfileId.value = profileId;
    try {
      await connect(profileId);
      await fetchDir(profileId, initialPath);
    } catch {
      // connect() error.value'yu zaten setledi; listing'i atla.
    }
  }

  async function navigate(path: string): Promise<void> {
    if (!boundProfileId.value) return;
    await fetchDir(boundProfileId.value, path);
  }

  async function refresh(): Promise<void> {
    if (!boundProfileId.value) return;
    // cwd "" olduğunda LocalAdapter root'unu yeniden istemek için onu koru.
    await fetchDir(boundProfileId.value, cwd.value);
  }

  async function up(): Promise<void> {
    if (!boundProfileId.value || !parent.value) return;
    await fetchDir(boundProfileId.value, parent.value);
  }

  async function setIncludeHidden(v: boolean): Promise<void> {
    includeHidden.value = v;
    await refresh();
  }

  function toggleSelect(path: string, mode: SelectionMode): void {
    const current = selection.value;
    if (mode === "single") {
      const next = new Set<string>();
      next.add(path);
      selection.value = next;
      anchorPath = path;
      return;
    }
    if (mode === "ctrl") {
      const next = new Set(current);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      selection.value = next;
      anchorPath = path;
      return;
    }
    if (mode === "shift") {
      const list = entries.value;
      const targetIndex = list.findIndex((e) => e.path === path);
      if (targetIndex < 0) return;
      const anchorIndex =
        anchorPath !== null ? list.findIndex((e) => e.path === anchorPath) : -1;
      if (anchorIndex < 0) {
        const next = new Set<string>();
        next.add(path);
        selection.value = next;
        anchorPath = path;
        return;
      }
      const [from, to] =
        anchorIndex <= targetIndex
          ? [anchorIndex, targetIndex]
          : [targetIndex, anchorIndex];
      const next = new Set(current);
      for (let i = from; i <= to; i++) {
        next.add(list[i].path);
      }
      selection.value = next;
      return;
    }
  }

  function clearSelection(): void {
    selection.value = new Set();
    anchorPath = null;
  }

  // Active profile değişimini otomatik takip et — UI'ın `bindToProfile`'ı manuel
  // çağırmasına gerek yok. ImmediateEffect ile mount sonrası ilk değer (varsa)
  // bağlanır.
  watch(
    () => activeProfileId.value,
    async (id) => {
      if (!id) {
        await disconnect();
        return;
      }
      const profile = activeProfile.value;
      if (!profile) {
        // Profile listesi henüz yüklenmediyse activeProfile null olabilir;
        // store loadAll bitince watch tekrar tetiklenir.
        return;
      }
      await bindToProfile(profile.id, initialPathFor(profile));
    },
    { immediate: true },
  );

  // activeProfileId set olduğunda activeProfile henüz hesaplanmamış olabilir
  // (store fetch + reactive update sırası). activeProfile değişimini de
  // izleyip aradaki gecikmeyi yakalıyoruz.
  watch(
    () => activeProfile.value,
    async (profile) => {
      if (!profile) return;
      if (boundProfileId.value === profile.id) return;
      await bindToProfile(profile.id, initialPathFor(profile));
    },
  );

  return {
    boundProfileId,
    cwd,
    entries,
    parent,
    loading,
    connecting,
    error,
    includeHidden,
    selection,
    capabilities,
    bindToProfile,
    navigate,
    refresh,
    up,
    setIncludeHidden,
    toggleSelect,
    clearSelection,
    disconnect,
  };
}
