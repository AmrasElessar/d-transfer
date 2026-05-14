import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";

export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

const STORAGE_KEY = "dtransfer.theme";

function readSystemPreference(): ResolvedTheme {
  if (typeof window === "undefined" || !window.matchMedia) return "dark";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export const useThemeStore = defineStore("theme", () => {
  const preference = ref<ThemePreference>("system");
  const systemTheme = ref<ResolvedTheme>(readSystemPreference());

  const resolvedTheme = computed<ResolvedTheme>(() =>
    preference.value === "system" ? systemTheme.value : preference.value,
  );

  function setPreference(next: ThemePreference) {
    preference.value = next;
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* storage disabled — boş geçildi */
    }
  }

  function init() {
    try {
      const stored = localStorage.getItem(STORAGE_KEY) as ThemePreference | null;
      if (stored === "light" || stored === "dark" || stored === "system") {
        preference.value = stored;
      }
    } catch {
      /* storage disabled */
    }

    if (typeof window !== "undefined" && window.matchMedia) {
      const mql = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = (e: MediaQueryListEvent) => {
        systemTheme.value = e.matches ? "dark" : "light";
      };
      mql.addEventListener("change", handler);
    }
  }

  watch(resolvedTheme, (val) => {
    if (typeof document === "undefined") return;
    document.documentElement.style.colorScheme = val;
  });

  return { preference, resolvedTheme, setPreference, init };
});
