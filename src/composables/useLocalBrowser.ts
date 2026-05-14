import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  ListLocalDirRequest,
  ListLocalDirResponse,
  LocalEntry,
  WireError,
} from "@/types";

/**
 * Yerel dosya sistemi browser state'i + IPC sarmalayıcısı.
 *
 * Adapter katmanından (LocalAdapter) ayrı; bu doğrudan UI'a hizmet eder ve
 * traversal jail uygulamaz — kullanıcı browser'da her yere gidebilmeli.
 *
 * Tauri runtime yoksa (saf `vite dev`) `error` set edilir ve `entries` boş
 * kalır; component crash etmez.
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

export function useLocalBrowser() {
  const cwd = ref<string>("");
  const entries = ref<LocalEntry[]>([]);
  const parent = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const includeHidden = ref(false);
  // Set<absolute path>. Set kullanmak Map'ten basit + has() O(1).
  const selection = ref<Set<string>>(new Set());

  // Shift-range için son tek seçilen "anchor" — Ctrl/Shift'siz tıklama
  // ile güncellenir. Selection clear edilirse anchor da düşer.
  let anchorPath: string | null = null;

  async function fetchDir(target: string): Promise<void> {
    if (!isTauriEnv()) {
      error.value = "Tauri runtime gerekli";
      entries.value = [];
      parent.value = null;
      cwd.value = "";
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      const request: ListLocalDirRequest = {
        path: target,
        includeHidden: includeHidden.value,
      };
      const resp = await invoke<ListLocalDirResponse>("list_local_dir", {
        request,
      });
      cwd.value = resp.path;
      entries.value = resp.entries;
      parent.value = resp.parent;
      // Yeni dizin → selection geçersiz; eski path'ler artık görünür değil.
      selection.value = new Set();
      anchorPath = null;
    } catch (err) {
      error.value = formatError(err);
      entries.value = [];
    } finally {
      loading.value = false;
    }
  }

  async function navigate(path: string): Promise<void> {
    await fetchDir(path);
  }

  async function refresh(): Promise<void> {
    if (cwd.value) await fetchDir(cwd.value);
  }

  async function up(): Promise<void> {
    if (parent.value) await fetchDir(parent.value);
  }

  async function home(): Promise<void> {
    if (!isTauriEnv()) {
      error.value = "Tauri runtime gerekli";
      return;
    }
    try {
      const h = await invoke<string | null>("home_dir");
      if (h) {
        await fetchDir(h);
      } else {
        // Home çözülemedi (Linux minimal env vs.) — Windows'ta C:\, POSIX'te /
        // yedeği. List_local_drives ilk bulduğu kökü kullanır.
        const drives = await invoke<string[]>("list_local_drives");
        if (drives.length > 0) await fetchDir(drives[0]);
      }
    } catch (err) {
      error.value = formatError(err);
    }
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
      // Ctrl-toggle son aktif tıklamayı anchor olarak günceller.
      anchorPath = path;
      return;
    }
    // Shift: anchor → current range, mevcut sıralama (entries order) baz alınır.
    if (mode === "shift") {
      const list = entries.value;
      const targetIndex = list.findIndex((e) => e.path === path);
      if (targetIndex < 0) return;
      // Anchor yoksa shift'i single gibi davran.
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
      // Shift anchor'ı değiştirmez (Windows/macOS Finder davranışı).
      return;
    }
  }

  function clearSelection(): void {
    selection.value = new Set();
    anchorPath = null;
  }

  onMounted(() => {
    // İlk render'da kullanıcının home dizinini göster — boş pane sıkıcı.
    void home();
  });

  return {
    cwd,
    entries,
    parent,
    loading,
    error,
    includeHidden,
    selection,
    navigate,
    refresh,
    up,
    home,
    toggleSelect,
    clearSelection,
  };
}
