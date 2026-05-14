import { defineStore } from "pinia";
import { ref } from "vue";

/**
 * İki pane'in (LocalPane / RemotePane) güncel kök dizinini paylaştığı küçük
 * köprü. Transfer butonları "kaynak pane'deki seçim + karşı pane'in cwd'si"
 * üzerinden çalıştığı için bu state'in tek bir yerde toplanması gerek.
 *
 * Pane'ler mount sonrası kendi browser state'lerini buraya watch ile aynalar;
 * pane composable'larına yeni ortak kaynak eklemekten daha az invazif.
 */
export const useTransferTargetStore = defineStore("transferTargets", () => {
  /** LocalPane'in canonical cwd'si (download hedefi). */
  const localCwd = ref<string>("");
  /** RemotePane'in cwd'si (upload hedefi). Boş = profile root. */
  const remoteCwd = ref<string>("");

  function setLocalCwd(path: string): void {
    localCwd.value = path;
  }
  function setRemoteCwd(path: string): void {
    remoteCwd.value = path;
  }

  return { localCwd, remoteCwd, setLocalCwd, setRemoteCwd };
});
