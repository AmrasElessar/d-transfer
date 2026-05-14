import { createI18n } from "vue-i18n";
import tr from "@/i18n/locales/tr.json";
import en from "@/i18n/locales/en.json";

export const SUPPORTED_LOCALES = ["tr", "en"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export type MessageSchema = typeof tr;

// `Legacy = false` üçüncü generic'i `i18n.global.locale`'ü
// `WritableComputedRef<SupportedLocale>` olarak doğru type'lar (Composer mode).
export const i18n = createI18n<{ message: MessageSchema }, SupportedLocale, false>({
  legacy: false,
  locale: "tr",
  fallbackLocale: "en",
  messages: { tr, en },
  missingWarn: false,
  fallbackWarn: false,
});
