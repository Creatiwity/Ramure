// Langue de l'interface : celle du système par défaut (français si le système est en français,
// anglais sinon), ou le choix mémorisé de l'utilisateur.
import { createI18n } from "vue-i18n";
import fr from "./fr";
import en from "./en";
import type { Locale } from "../lib/format";

export type LangPref = "" | Locale;

const KEY = "ramure.lang";

export function systemLang(): Locale {
  const langs = typeof navigator !== "undefined" ? (navigator.languages?.length ? navigator.languages : [navigator.language]) : [];
  return (langs[0] ?? "en").toLowerCase().startsWith("fr") ? "fr" : "en";
}

export function loadLangPref(): LangPref {
  try {
    const v = localStorage.getItem(KEY);
    return v === "fr" || v === "en" ? v : "";
  } catch {
    return "";
  }
}

export const i18n = createI18n({
  legacy: false,
  locale: loadLangPref() || systemLang(),
  fallbackLocale: "en",
  messages: { fr, en },
});

export function setLangPref(pref: LangPref) {
  try {
    if (pref) localStorage.setItem(KEY, pref);
    else localStorage.removeItem(KEY);
  } catch {
    /* préférence non mémorisée */
  }
  const lang = pref || systemLang();
  i18n.global.locale.value = lang;
  document.documentElement.lang = lang;
}

/** Message traduit pour une erreur du cœur (`{ code, detail }`) ou toute autre erreur. */
export function errorText(e: unknown): string {
  const t = i18n.global.t;
  if (e && typeof e === "object" && "code" in e) {
    const { code, detail } = e as { code: string; detail?: string };
    const key = `errors.${code}`;
    return i18n.global.te(key) ? t(key, { detail: detail ?? "" }) : t("errors.unknown", { detail: detail || code });
  }
  const msg = e instanceof Error ? e.message : String(e);
  return t("errors.unknown", { detail: msg });
}
