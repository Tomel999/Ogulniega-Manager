import { createI18n } from "vue-i18n";
import en from "./locales/en.json";
import pl from "./locales/pl.json";
import ru from "./locales/ru.json";

export const SUPPORTED_LOCALES = [
  { value: "en", label: "English" },
  { value: "pl", label: "Polski" },
  { value: "ru", label: "Русский" },
];

const saved = localStorage.getItem("locale");
const defaultLocale = saved && SUPPORTED_LOCALES.some((l) => l.value === saved) ? saved : "en";

const i18n = createI18n({
  legacy: false,
  locale: defaultLocale,
  fallbackLocale: "en",
  messages: { en, pl, ru },
});

export default i18n;
