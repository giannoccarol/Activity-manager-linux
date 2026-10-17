// Interface language: Italian and English, from two JSON dictionaries.
// t("section.key", { name: "x" }) looks up the key, falls back to English, and fills {placeholders}.
// A value with "one"/"other" is a plural: pass { count } to pick the form.
import it from "../locales/it.json";
import en from "../locales/en.json";

export type Lang = "it" | "en";
export type LangPref = Lang | "auto";
type Tree = { [k: string]: string | Tree };
const DICT: Record<Lang, Tree> = { it: it as Tree, en: en as Tree };

function browserLang(): Lang {
  const l = (typeof navigator !== "undefined" ? (navigator.languages?.[0] ?? navigator.language) : "en") ?? "en";
  return l.toLowerCase().startsWith("it") ? "it" : "en";
}

class I18n {
  lang = $state<Lang>(browserLang());
  /** system language (from the backend in the app, from the browser otherwise) */
  system: Lang = browserLang();
  /** BCP 47 tag for numbers and dates */
  get tag() {
    return this.lang === "it" ? "it-IT" : "en-US";
  }
}
export const i18n = new I18n();

export function applyLanguage(pref: LangPref) {
  i18n.lang = pref === "auto" ? i18n.system : pref;
  if (typeof document !== "undefined") document.documentElement.lang = i18n.lang;
}

function lookup(tree: Tree, key: string): string | Tree | undefined {
  let v: string | Tree | undefined = tree;
  for (const part of key.split(".")) {
    if (typeof v !== "object" || v === null) return undefined;
    v = v[part];
  }
  return v;
}

const plurals = new Map<string, Intl.PluralRules>();

export function t(key: string, params?: Record<string, string | number>): string {
  const lang = i18n.lang;
  let v = lookup(DICT[lang], key) ?? lookup(DICT.en, key) ?? lookup(DICT.it, key);
  if (v && typeof v === "object" && params && typeof params.count === "number") {
    let pr = plurals.get(lang);
    if (!pr) plurals.set(lang, (pr = new Intl.PluralRules(lang === "it" ? "it-IT" : "en-US")));
    v = v[pr.select(params.count)] ?? v.other;
  }
  if (typeof v !== "string") return key;
  return params ? v.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m)) : v;
}
