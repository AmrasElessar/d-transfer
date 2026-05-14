import { defineStore } from "pinia";
import { ref } from "vue";
import { i18n, SUPPORTED_LOCALES, type SupportedLocale } from "@/i18n";

const STORAGE_KEY = "dtransfer.locale";

function detectSystemLocale(): SupportedLocale {
  if (typeof navigator === "undefined") return "tr";
  const candidates = [navigator.language, ...(navigator.languages ?? [])]
    .map((tag) => tag.toLowerCase().slice(0, 2));
  for (const tag of candidates) {
    if (SUPPORTED_LOCALES.includes(tag as SupportedLocale)) {
      return tag as SupportedLocale;
    }
  }
  return "tr";
}

export const useLocaleStore = defineStore("locale", () => {
  const locale = ref<SupportedLocale>("tr");

  function setLocale(next: SupportedLocale) {
    if (!SUPPORTED_LOCALES.includes(next)) return;
    locale.value = next;
    i18n.global.locale.value = next;
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* storage disabled */
    }
  }

  function init() {
    let initial: SupportedLocale | null = null;
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored && SUPPORTED_LOCALES.includes(stored as SupportedLocale)) {
        initial = stored as SupportedLocale;
      }
    } catch {
      /* storage disabled */
    }
    setLocale(initial ?? detectSystemLocale());
  }

  return { locale, setLocale, init };
});
