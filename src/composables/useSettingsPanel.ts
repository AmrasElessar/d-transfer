import { ref } from "vue";

// Module-level singleton state — birden fazla yerden useSettingsPanel() çağrılsa
// da hep aynı `isOpen` ref'ini paylaşır.
const isOpen = ref(false);

export function useSettingsPanel() {
  return {
    isOpen,
    open() {
      isOpen.value = true;
    },
    close() {
      isOpen.value = false;
    },
    toggle() {
      isOpen.value = !isOpen.value;
    },
  };
}
