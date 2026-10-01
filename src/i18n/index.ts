import { createI18n } from "vue-i18n";
import idID from "./locales/id-ID";

export const DEFAULT_LOCALE = "id-ID" as const;

export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: DEFAULT_LOCALE,
  fallbackLocale: DEFAULT_LOCALE,
  messages: { [DEFAULT_LOCALE]: idID },
  numberFormats: {
    [DEFAULT_LOCALE]: {
      currency: {
        style: "currency",
        currency: "IDR",
        maximumFractionDigits: 0,
      },
      compactCurrency: {
        style: "currency",
        currency: "IDR",
        maximumFractionDigits: 0,
        notation: "compact",
      },
    },
  },
  datetimeFormats: {
    [DEFAULT_LOCALE]: {
      short: { day: "numeric", month: "short", year: "numeric" },
      time: { hour: "2-digit", minute: "2-digit" },
    },
  },
});

export const translate = i18n.global.t;
