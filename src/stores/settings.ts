import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, AppSettingsPatch } from "@/types";

const DEFAULTS: AppSettings = {
  schemaVersion: 1,
  defaultDownloadDir: null,
  maxConcurrentTransfers: 1,
  defaultChunkSizeMb: 8,
  defaultMaxInflightMb: 64,
  bandwidthLimitBps: null,
  verifyChecksum: "sha256",
  fsyncPolicy: "dataOnly",
  autoUpdate: true,
  telemetry: false,
};

function isTauriEnv(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * AppSettings backend ile senkron Pinia store.
 *
 * - `loadOnce()`: ilk mount'ta IPC ile diskten yükler.
 * - `apply(patch)`: IPC üzerinden backend'i günceller, dönen snapshot'ı state'e set eder.
 *
 * Tauri-dışı ortamda (saf `pnpm dev` tarayıcı) `defaults`'a düşer; IPC çağrıları
 * sessizce ignore edilir (warn log'lu).
 */
export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<AppSettings>({ ...DEFAULTS });
  const loaded = ref(false);
  const lastError = ref<string | null>(null);

  async function loadOnce(): Promise<void> {
    if (loaded.value) return;
    if (!isTauriEnv()) {
      loaded.value = true;
      return;
    }
    try {
      const fetched = await invoke<AppSettings>("get_settings");
      settings.value = fetched;
      loaded.value = true;
      lastError.value = null;
    } catch (err) {
      lastError.value = String(err);
      console.warn("[settings] get_settings failed:", err);
    }
  }

  async function apply(patch: AppSettingsPatch): Promise<AppSettings> {
    if (!isTauriEnv()) {
      // Tauri-dışı: in-memory uygula, persist yok.
      const merged = { ...settings.value, ...(patch as Partial<AppSettings>) };
      settings.value = merged;
      return merged;
    }
    try {
      const updated = await invoke<AppSettings>("update_settings", { patch });
      settings.value = updated;
      lastError.value = null;
      return updated;
    } catch (err) {
      lastError.value = String(err);
      throw err;
    }
  }

  return { settings, loaded, lastError, loadOnce, apply };
});
